use core::{cell::OnceCell, ptr::NonNull};
use librashader_presets::ShaderFeatures;
use std::sync::RwLock;

use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType,
    NSWindow, NSWindowStyleMask,
};
use objc2_metal::{
    MTLBlitCommandEncoder, MTLClearColor, MTLTexture, MTLTextureDescriptor, MTLTextureUsage,
};

use objc2_foundation::{
    ns_string, MainThreadMarker, NSDate, NSNotification, NSObject, NSObjectProtocol, NSPoint,
    NSRect, NSSize,
};

use objc2_metal::{
    MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice,
    MTLDrawable, MTLLibrary, MTLPrimitiveType, MTLRenderCommandEncoder,
    MTLRenderPipelineDescriptor, MTLRenderPipelineState,
};
use objc2_metal_kit::{MTKView, MTKViewDelegate};

use librashader_common::Viewport;
use librashader_presets::ShaderPreset;
use librashader_runtime_mtl::FilterChainMetal;
use objc2::__framework_prelude::Retained;
use objc2::{
    declare_class, define_class, msg_send, msg_send_id, rc::Id, runtime::ProtocolObject, ClassType,
    DeclaredClass, MainThreadOnly,
};

#[rustfmt::skip]
const SHADERS: &str = r#"
    #include <metal_stdlib>

    struct SceneProperties {
        float time;
    };

    struct VertexInput {
        metal::packed_float3 position;
        metal::packed_float3 color;
    };

    struct VertexOutput {
        metal::float4 position [[position]];
        metal::float4 color;
    };

    vertex VertexOutput vertex_main(
        device const SceneProperties& properties [[buffer(0)]],
        device const VertexInput* vertices [[buffer(1)]],
        uint vertex_idx [[vertex_id]]
    ) {
        VertexOutput out;
        VertexInput in = vertices[vertex_idx];
        out.position =
            metal::float4(
                metal::float2x2(
                    metal::cos(properties.time), -metal::sin(properties.time),
                    metal::sin(properties.time),  metal::cos(properties.time)
                ) * in.position.xy,
                in.position.z,
                1);
        out.color = metal::float4(in.color, 1);
        return out;
    }

    fragment metal::float4 fragment_main(VertexOutput in [[stage_in]]) {
        return in.color;
    }
"#;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct SceneProperties {
    pub time: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct VertexInput {
    pub position: Position,
    pub color: Color,
}

#[derive(Copy, Clone)]
// NOTE: this has the same ABI as `MTLPackedFloat3`
#[repr(C)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Copy, Clone)]
// NOTE: this has the same ABI as `MTLPackedFloat3`
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

macro_rules! idcell {
    ($name:ident => $this:expr) => {
        $this.ivars().$name.set($name).expect(&format!(
            "ivar should not already be initialized: `{}`",
            stringify!($name)
        ));
    };
    ($name:ident <= $this:expr) => {
        #[rustfmt::skip]
        let Some($name) = $this.ivars().$name.get() else {
            unreachable!(
                "ivar should be initialized: `{}`",
                stringify!($name)
            )
        };
    };
}

// declare the desired instance variables
struct Ivars {
    start_date: Retained<NSDate>,
    command_queue: OnceCell<Retained<ProtocolObject<dyn MTLCommandQueue>>>,
    pipeline_state: OnceCell<Retained<ProtocolObject<dyn MTLRenderPipelineState>>>,
    filter_chain: OnceCell<RwLock<FilterChainMetal>>,
    window: OnceCell<Retained<NSWindow>>,
}

// declare the Objective-C class machinery
define_class!(
    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Main thread only mutability is correct, since this is an application delegate.
    // - `Delegate` does not implement `Drop`.
    #[unsafe(super = NSObject)]
    #[name = "Delegate"]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    // define the delegate methods for the `NSApplicationDelegate` protocol
    unsafe impl NSApplicationDelegate for Delegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        #[allow(non_snake_case)]
        unsafe fn applicationDidFinishLaunching(&self, _notification: &NSNotification) {
            let mtm = MainThreadMarker::from(self);
            // create the app window
            let window = {
                let content_rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(768., 768.));
                let style = NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Resizable
                    | NSWindowStyleMask::Titled;
                let backing_store_type = NSBackingStoreType::Buffered;
                let flag = false;
                unsafe {
                    NSWindow::initWithContentRect_styleMask_backing_defer(
                        mtm.alloc(),
                        content_rect,
                        style,
                        backing_store_type,
                        flag,
                    )
                }
            };

            // get the default device
            let device =
                { MTLCreateSystemDefaultDevice().expect("Failed to get system default device.") };

            // create the command queue
            let command_queue = device
                .newCommandQueue()
                .expect("Failed to create a command queue.");

            // create the metal view
            let mtk_view = {
                let frame_rect = window.frame();
                unsafe { MTKView::initWithFrame_device(mtm.alloc(), frame_rect, Some(&device)) }
            };

            // create the pipeline descriptor
            let pipeline_descriptor = MTLRenderPipelineDescriptor::new();

            unsafe {
                pipeline_descriptor
                    .colorAttachments()
                    .objectAtIndexedSubscript(0)
                    .setPixelFormat(mtk_view.colorPixelFormat());
            }

            // compile the shaders
            let library = device
                .newLibraryWithSource_options_error(ns_string!(SHADERS), None)
                .expect("Failed to create a library.");

            // configure the vertex shader
            let vertex_function = library.newFunctionWithName(ns_string!("vertex_main"));
            pipeline_descriptor.setVertexFunction(vertex_function.as_deref());

            // configure the fragment shader
            let fragment_function = library.newFunctionWithName(ns_string!("fragment_main"));
            pipeline_descriptor.setFragmentFunction(fragment_function.as_deref());

            // create the pipeline state
            let pipeline_state = device
                .newRenderPipelineStateWithDescriptor_error(&pipeline_descriptor)
                .expect("Failed to create a pipeline state.");

            let preset = ShaderPreset::try_parse(
                "../test/shaders_slang/anti-aliasing/smaa.slangp",
                ShaderFeatures::empty(),
            )
            .unwrap();

            let filter_chain =
                FilterChainMetal::load_from_preset(preset, &command_queue, None).unwrap();

            let filter_chain = RwLock::new(filter_chain);

            // configure the metal view delegate
            unsafe {
                let object = ProtocolObject::from_ref(self);
                mtk_view.setDelegate(Some(object));
            }

            // configure the window
            window.setContentView(Some(&mtk_view));
            window.center();
            window.setTitle(ns_string!("metal example"));
            window.makeKeyAndOrderFront(None);

            // initialize the delegate state
            idcell!(command_queue => self);
            idcell!(pipeline_state => self);
            idcell!(filter_chain => self);
            idcell!(window => self);
        }
    }

    // define the delegate methods for the `MTKViewDelegate` protocol
    unsafe impl MTKViewDelegate for Delegate {
        #[unsafe(method(drawInMTKView:))]
        #[allow(non_snake_case)]
        unsafe fn drawInMTKView(&self, mtk_view: &MTKView) {
            idcell!(command_queue <= self);
            idcell!(pipeline_state <= self);
            idcell!(filter_chain <= self);

            unsafe {
                mtk_view.setFramebufferOnly(false);
                mtk_view.setClearColor(MTLClearColor {
                    red: 0.3,
                    blue: 0.5,
                    green: 0.3,
                    alpha: 0.0,
                });
            }

            // FIXME: icrate `MTKView` doesn't have a generated binding for `currentDrawable` yet
            // (because it needs a definition of `CAMetalDrawable`, which we don't support yet) so
            // we have to use a raw `msg_send_id` call here instead.
            let current_drawable: Option<Retained<ProtocolObject<dyn MTLDrawable>>> =
                msg_send_id![mtk_view, currentDrawable];

            // prepare for drawing
            let Some(current_drawable) = current_drawable else {
                return;
            };
            let Some(command_buffer) = command_queue.commandBuffer() else {
                return;
            };
            let Some(pass_descriptor) = (unsafe { mtk_view.currentRenderPassDescriptor() }) else {
                return;
            };

            let Some(encoder) = command_buffer.renderCommandEncoderWithDescriptor(&pass_descriptor)
            else {
                return;
            };

            // compute the scene properties
            let scene_properties_data = &SceneProperties {
                time: unsafe { self.ivars().start_date.timeIntervalSinceNow() } as f32,
            };
            // write the scene properties to the vertex shader argument buffer at index 0
            let scene_properties_bytes = NonNull::from(scene_properties_data);
            unsafe {
                encoder.setVertexBytes_length_atIndex(
                    scene_properties_bytes.cast::<core::ffi::c_void>(),
                    core::mem::size_of_val(scene_properties_data),
                    0,
                )
            };

            // compute the triangle geometry
            let vertex_input_data: &[VertexInput] = &[
                VertexInput {
                    position: Position {
                        x: -f32::sqrt(3.0) / 4.0,
                        y: -0.25,
                        z: 0.,
                    },
                    color: Color {
                        r: 1.,
                        g: 0.,
                        b: 0.,
                    },
                },
                VertexInput {
                    position: Position {
                        x: f32::sqrt(3.0) / 4.0,
                        y: -0.25,
                        z: 0.,
                    },
                    color: Color {
                        r: 0.,
                        g: 1.,
                        b: 0.,
                    },
                },
                VertexInput {
                    position: Position {
                        x: 0.,
                        y: 0.5,
                        z: 0.,
                    },
                    color: Color {
                        r: 0.,
                        g: 0.,
                        b: 1.,
                    },
                },
            ];
            // write the triangle geometry to the vertex shader argument buffer at index 1
            let vertex_input_bytes = NonNull::from(vertex_input_data);
            unsafe {
                encoder.setVertexBytes_length_atIndex(
                    vertex_input_bytes.cast::<core::ffi::c_void>(),
                    core::mem::size_of_val(vertex_input_data),
                    1,
                )
            };

            // configure the encoder with the pipeline and draw the triangle
            encoder.setRenderPipelineState(pipeline_state);
            unsafe {
                encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3)
            };
            encoder.endEncoding();

            unsafe {
                let mut filter_chain = filter_chain.write().unwrap();
                let texture = pass_descriptor
                    .colorAttachments()
                    .objectAtIndexedSubscript(0)
                    .texture()
                    .unwrap();

                let tex_desc =
                    MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                        texture.pixelFormat(),
                        texture.width(),
                        texture.height(),
                        false,
                    );

                tex_desc.setUsage(MTLTextureUsage::RenderTarget);
                //  let frontbuffer = command_queue
                // .device()
                // .newTextureWithDescriptor(&tex_desc)
                // .unwrap();

                let backbuffer = command_queue
                    .device()
                    .newTextureWithDescriptor(&tex_desc)
                    .unwrap();

                //   let blit = command_buffer
                // .blitCommandEncoder()
                // .unwrap();
                // blit.copyFromTexture_toTexture(&texture, &frontbuffer);
                // blit.endEncoding();

                filter_chain
                    .frame(
                        &texture,
                        &Viewport::new_render_target_sized_origin(backbuffer.as_ref(), None)
                            .expect("viewport"),
                        &command_buffer,
                        1,
                        None,
                    )
                    .expect("frame");

                let blit = command_buffer.blitCommandEncoder().unwrap();
                blit.copyFromTexture_toTexture(&backbuffer, &texture);
                blit.endEncoding();
            }

            // schedule the command buffer for display and commit
            command_buffer.presentDrawable(&current_drawable);
            command_buffer.commit();
        }

        #[unsafe(method(mtkView:drawableSizeWillChange:))]
        #[allow(non_snake_case)]
        unsafe fn mtkView_drawableSizeWillChange(&self, _view: &MTKView, _size: NSSize) {
            // println!("mtkView_drawableSizeWillChange");
        }
    }
);

impl Delegate {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(Ivars {
            start_date: unsafe { NSDate::now() },
            command_queue: OnceCell::default(),
            pipeline_state: OnceCell::default(),
            filter_chain: OnceCell::default(),
            window: OnceCell::default(),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn main() {
    let mtm = MainThreadMarker::new().unwrap();
    // configure the app
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    // configure the application delegate
    let delegate = Delegate::new(mtm);
    let object = ProtocolObject::from_ref(&*delegate);
    app.setDelegate(Some(object));

    // run the app
    unsafe { app.run() };
}
