use std::borrow::Cow;
use std::num::NonZeroU32;
use std::sync::Arc;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, Device, PipelineLayout, PushConstantRange, SamplerBindingType, ShaderModule, ShaderSource, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexAttribute, VertexBufferLayout};
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::back::wgsl::NagaWgslContext;
use librashader_reflect::reflect::semantics::BufferReflection;
use librashader_reflect::reflect::ShaderReflection;
use crate::util;

pub struct WgpuGraphicsPipeline {
    vertex: ShaderModule,
    fragment: ShaderModule
}

pub struct PipelineLayoutObjects {
    pub layout: PipelineLayout,
    pub bind_group_layouts: Vec<BindGroupLayout>
}

impl PipelineLayoutObjects {
    pub fn new(
        reflection: &ShaderReflection,
        device: &Device
    ) -> Self {

        let mut bind_group_layouts = Vec::new();

        let mut main_bindings = Vec::new();
        let mut sampler_bindings = Vec::new();

        let mut push_constant_range = Vec::new();

        if let Some(push_meta) = reflection.push_constant.as_ref() && !push_meta.stage_mask.is_empty() {
            let push_mask = util::binding_stage_to_wgpu_stage(push_meta.stage_mask);

            if let Some(binding) = push_meta.binding {
                main_bindings.push(BindGroupLayoutEntry {
                    binding,
                    visibility: push_mask,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(push_meta.size as u64),
                    },
                    count: None,
                });
            } else {
                push_constant_range.push(PushConstantRange {
                    stages: push_mask,
                    range: 0..push_meta.size,
                })
            }
        }

        if let Some(ubo_meta) = reflection.ubo.as_ref() && !ubo_meta.stage_mask.is_empty() {
            let ubo_mask = util::binding_stage_to_wgpu_stage(ubo_meta.stage_mask);
            main_bindings.push(BindGroupLayoutEntry {
                binding: ubo_meta.binding,
                visibility: ubo_mask,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(ubo_meta.size as u64),
                },
                count: None,
            });
        }

        for texture in reflection.meta.texture_meta.values() {
            main_bindings.push(BindGroupLayoutEntry {
                binding: texture.binding,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });

            sampler_bindings.push(BindGroupLayoutEntry {
                binding: texture.binding,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            })
        }
        let main_bind_group = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bind group 0"),
            entries: &main_bindings,
        });

        let sampler_bind_group = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bind group 1"),
            entries: &sampler_bindings,
        });

        bind_group_layouts.push(main_bind_group);
        bind_group_layouts.push(sampler_bind_group);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shader pipeline layout"),
            bind_group_layouts: &bind_group_layouts.as_ref(),
            push_constant_ranges: &push_constant_range.as_ref(),
        });

        Self {
            layout,
            bind_group_layouts
        }
    }
}



impl WgpuGraphicsPipeline {
    pub fn new(
        device: &Device,
        shader_assembly: &ShaderCompilerOutput<String, NagaWgslContext>,
        reflection: &ShaderReflection,
        render_pass_format: TextureFormat,
    ) -> Self {
        let vertex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vertex"),
                source: ShaderSource::Wgsl(Cow::from(&shader_assembly.vertex))
            });

        let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fragment"),
            source: ShaderSource::Wgsl(Cow::from(&shader_assembly.fragment))
        });

        let layout = PipelineLayoutObjects::new(reflection, device);

        let vao_layout = VertexBufferLayout {
            array_stride: 4 * std::mem::size_of::<f32>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format:  wgpu::VertexFormat::Float32x2,
                    offset: (2 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 1,
                },
            ],
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout.layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: &shader_assembly
                    .context
                    .vertex
                    .entry_points[0]
                    .name,
                buffers: &[
                    vao_layout
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: &shader_assembly
                    .context
                    .fragment
                    .entry_points[0]
                    .name,
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_pass_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            vertex,
            fragment,
        }
    }
}