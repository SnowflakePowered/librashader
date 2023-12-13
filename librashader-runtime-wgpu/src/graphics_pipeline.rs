use std::borrow::Cow;
use std::num::NonZeroU32;
use std::sync::Arc;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, Device, PipelineLayout, PushConstantRange, ShaderModule, ShaderSource, ShaderStages, TextureFormat};
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::UboReflection;
use librashader_reflect::reflect::ShaderReflection;
use crate::util;

pub struct WgpuGraphicsPipeline {
    vertex: ShaderModule,
    fragment: ShaderModule
}

pub struct PipelineLayoutObjects {
    pub layout: PipelineLayout,
    pub bind_groups: Vec<BindGroup>,
    pub bind_group_layouts: Vec<BindGroupLayout>
}

pub fn add_ubo_binding(&mut self, ubo_meta: Option<&UboReflection>) {

}

pub fn add_texture_bindings<'a>(&mut self, textures: impl Iterator<Item = &'a TextureBinding>) {
    let texture_mask = vk::ShaderStageFlags::FRAGMENT;
    for texture in textures {
        self.layout_bindings.push(vk::DescriptorSetLayoutBinding {
            binding: texture.binding,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
            stage_flags: texture_mask,
            p_immutable_samplers: std::ptr::null(),
        });

        self.pool_sizes.push(vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: self.replicas,
        })
    }
}

impl PipelineLayoutObjects {
    pub fn new(
        reflection: &ShaderReflection,
        device: &Device
    ) -> Self {
        let push_constant_range = reflection.push_constant
            .as_ref()
            .map(|push_constant| {
                let stage_mask = util::binding_stage_to_wgpu_stage(push_constant.stage_mask);
                [PushConstantRange {
                    stages: stage_mask,
                    range: 0..push_constant.size,
                }]
            });

        let mut bind_group_layouts = Vec::new();

        if let Some(ubo_meta) = reflection.ubo.as_ref() && !ubo_meta.stage_mask.is_empty() {
            let ubo_mask = util::binding_stage_to_wgpu_stage(ubo_meta.stage_mask);

            let ubo_bind_group = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("ubo bind group"),
                entries: &[BindGroupLayoutEntry {
                    binding: ubo_meta.binding,
                    visibility: ubo_mask,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(ubo_meta.size as u64),
                    },
                    count: Some(NonZeroU32::MIN),
                }],
            });

            bind_group_layouts.push(ubo_bind_group)
        }

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shader pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: push_constant_range.as_ref()
                .unwrap_or(&[]),
        });

        Self {
            layout,
            bind_groups: vec![],
            bind_group_layouts
        }
    }
}



impl WgpuGraphicsPipeline {
    pub fn new(
        device: &Device,
        shader_assembly: &ShaderCompilerOutput<Vec<u32>>,
        reflection: &ShaderReflection,
        render_pass_format: TextureFormat,
    ) -> Self {
        // todo: naga shaders man.
        let vertex = unsafe {
            device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                label: Some("vertex"),
                source: Cow::from(&shader_assembly.vertex),
            })
        };

        let fragment = unsafe {
            device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                label: Some("fragment"),
                source: Cow::from(&shader_assembly.fragment),
            })
        };




        Self {
            vertex,
            fragment
        }
    }
}