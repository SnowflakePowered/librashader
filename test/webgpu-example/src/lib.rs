use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use librashader_common::{Size, Viewport};
use librashader_pack::ShaderPresetPack;
use librashader_runtime_wgpu::{
    options::FilterChainOptionsWgpu, FilterChainWgpu, WgpuOutputView,
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::util::DeviceExt;

const CRT_ROYALE_PACK: &[u8] = include_bytes!("../assets/crt-royale.wgsl.slangpkg");

const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TriUniforms {
    // mat4-aligned: 16 floats, with the 2x2 rotation in the upper-left.
    matrix: [[f32; 4]; 4],
}

struct Demo {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    surface_size: (u32, u32),

    triangle_pipeline: wgpu::RenderPipeline,
    triangle_uniforms: wgpu::Buffer,
    triangle_bind_group: wgpu::BindGroup,

    offscreen: wgpu::Texture,

    filter_chain: FilterChainWgpu,
    frame_count: usize,
    start_time: f64,
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Entry point invoked from JS. The canvas element ID is passed in so the demo
/// can attach its WebGPU surface to it.
#[wasm_bindgen]
pub async fn run(canvas_id: String) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas: HtmlCanvasElement = document
        .get_element_by_id(&canvas_id)
        .ok_or_else(|| JsValue::from_str("canvas not found"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("element is not a canvas"))?;

    let width = canvas.width().max(1);
    let height = canvas.height().max(1);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    // Building a `Surface<'static>` from a canvas requires SurfaceTarget::Canvas;
    // wgpu copies the handle internally.
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| JsValue::from_str(&format!("create_surface: {e:?}")))?;

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .map_err(|e| JsValue::from_str(&format!("request_adapter: {e:?}")))?;

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("librashader web-demo device"),
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                .using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|e| JsValue::from_str(&format!("request_device: {e:?}")))?;

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);

    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        },
    );

    // ---- triangle pipeline ----------------------------------------------------

    let triangle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("triangle"),
        source: wgpu::ShaderSource::Wgsl(TRIANGLE_WGSL.into()),
    });

    let triangle_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("triangle uniforms"),
        contents: bytemuck::bytes_of(&TriUniforms {
            matrix: IDENTITY_MATRIX,
        }),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let triangle_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("triangle bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let triangle_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("triangle bg"),
        layout: &triangle_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: triangle_uniforms.as_entire_binding(),
        }],
    });

    let triangle_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("triangle layout"),
        bind_group_layouts: &[Some(&triangle_bgl)],
        immediate_size: 0,
    });

    let triangle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("triangle pipeline"),
        layout: Some(&triangle_layout),
        vertex: wgpu::VertexState {
            module: &triangle_shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &triangle_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: OFFSCREEN_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    let offscreen = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: OFFSCREEN_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // ---- filter chain --------------------------------------------------------

    let preset: ShaderPresetPack = rmp_serde::from_slice(CRT_ROYALE_PACK)
        .map_err(|e| JsValue::from_str(&format!("decode preset pack: {e}")))?;

    let filter_chain = FilterChainWgpu::load_from_pack(
        preset,
        &device,
        &queue,
        Some(&FilterChainOptionsWgpu {
            // PipelineCache requires a feature we didn't request; opt out.
            enable_cache: false,
            force_no_mipmaps: false,
            adapter_info: None,
        }),
    )
    .map_err(|e| JsValue::from_str(&format!("load filter chain: {e}")))?;

    let demo = Arc::new(std::sync::Mutex::new(Demo {
        device,
        queue,
        surface,
        surface_format,
        surface_size: (width, height),
        triangle_pipeline,
        triangle_uniforms,
        triangle_bind_group,
        offscreen,
        filter_chain,
        frame_count: 0,
        start_time: now_millis(),
    }));

    // Schedule the render loop via requestAnimationFrame.
    schedule_frame(demo);
    Ok(())
}

fn schedule_frame(demo: Arc<std::sync::Mutex<Demo>>) {
    let demo_for_cb = demo.clone();
    let cb = Closure::once_into_js(move || {
        if let Ok(mut d) = demo_for_cb.lock() {
            d.render();
        }
        schedule_frame(demo_for_cb.clone());
    });

    let window = web_sys::window().expect("no window");
    let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
}

impl Demo {
    fn render(&mut self) {
        // Update rotation.
        let t = ((now_millis() - self.start_time) / 1000.0) as f32;
        let (s, c) = (t.sin(), t.cos());
        let aspect = self.surface_size.0 as f32 / self.surface_size.1 as f32;
        let mut mat = IDENTITY_MATRIX;
        mat[0] = [c / aspect, -s, 0.0, 0.0];
        mat[1] = [s / aspect, c, 0.0, 0.0];
        self.queue.write_buffer(
            &self.triangle_uniforms,
            0,
            bytemuck::bytes_of(&TriUniforms { matrix: mat }),
        );

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return,
        };
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut cmd = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });

        // 1. Render triangle into offscreen.
        let offscreen_view = self
            .offscreen
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut pass = cmd.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("triangle pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &offscreen_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.triangle_pipeline);
            pass.set_bind_group(0, &self.triangle_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // 2. Run the filter chain into the surface.
        let viewport_size = Size::new(self.surface_size.0, self.surface_size.1);
        let viewport = Viewport {
            x: 0.0,
            y: 0.0,
            mvp: None,
            output: WgpuOutputView::new_from_raw(&frame_view, viewport_size, self.surface_format),
            size: viewport_size,
        };

        let _ = self.filter_chain.frame(
            &self.offscreen,
            &viewport,
            &mut cmd,
            self.frame_count,
            None,
        );
        self.frame_count = self.frame_count.wrapping_add(1);

        self.queue.submit([cmd.finish()]);
        frame.present();
    }
}

const IDENTITY_MATRIX: [[f32; 4]; 4] = {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
};

fn now_millis() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

const TRIANGLE_WGSL: &str = r#"
struct Uniforms {
    matrix: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>( 0.0,  0.6),
        vec2<f32>(-0.6, -0.5),
        vec2<f32>( 0.6, -0.5),
    );
    var colors = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.2, 0.2),
        vec3<f32>(0.2, 1.0, 0.3),
        vec3<f32>(0.2, 0.3, 1.0),
    );
    let p = positions[idx];
    var out: VsOut;
    out.pos = u.matrix * vec4<f32>(p, 0.0, 1.0);
    out.color = colors[idx];
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;
