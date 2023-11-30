use std::borrow::Cow;
use std::sync::Arc;
use wgpu::{Device, ShaderModule, ShaderSource, TextureFormat};
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::ShaderReflection;

pub struct WgpuGraphicsPipeline {
    vertex: ShaderModule,
    fragment: ShaderModule
}

impl WgpuGraphicsPipeline {
    pub fn new(
        device: &Device,
        shader_assembly: &ShaderCompilerOutput<Vec<u32>>,
        reflection: &ShaderReflection,
        render_pass_format: TextureFormat,
    ) -> Self {
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

        // let render_pipeline_layout =
        //     device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //         label: Some("Render Pipeline Layout"),
        //         bind_group_layouts: &[],
        //         push_constant_ranges: &[],
        //     });
        Self {
            vertex,
            fragment
        }
    }
}