use crate::{error, util};
use ash::vk;

use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{TextureBinding, UboReflection};
use librashader_reflect::reflect::ShaderReflection;
use std::ffi::CStr;

const ENTRY_POINT: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };

pub struct PipelineDescriptors {
    pub replicas: u32,
    pub layout_bindings: Vec<vk::DescriptorSetLayoutBinding>,
    pub pool_sizes: Vec<vk::DescriptorPoolSize>,
}

impl PipelineDescriptors {
    pub fn new(duplicates: u32) -> Self {
        Self {
            replicas: duplicates,
            layout_bindings: vec![],
            pool_sizes: vec![],
        }
    }

    pub fn add_ubo_binding(&mut self, ubo_meta: Option<&UboReflection>) {
        if let Some(ubo_meta) = ubo_meta && !ubo_meta.stage_mask.is_empty() {
            let ubo_mask = util::binding_stage_to_vulkan_stage(ubo_meta.stage_mask);

            self.layout_bindings.push(vk::DescriptorSetLayoutBinding {
                binding: ubo_meta.binding,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: ubo_mask,
                p_immutable_samplers: std::ptr::null(),
            });

            self.pool_sizes.push(vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: self.replicas,
            })
        }
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

    pub fn bindings(&self) -> &[vk::DescriptorSetLayoutBinding] {
        self.layout_bindings.as_ref()
    }

    pub fn create_descriptor_set_layout(
        &self,
        device: &ash::Device,
    ) -> error::Result<vk::DescriptorSetLayout> {
        unsafe {
            let layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .bindings(self.bindings())
                    .build(),
                None,
            )?;
            Ok(layout)
        }
    }
}

pub struct PipelineLayoutObjects {
    pub layout: vk::PipelineLayout,
    pub pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub descriptor_set_layout: [vk::DescriptorSetLayout; 1],
}

impl PipelineLayoutObjects {
    pub fn new(
        reflection: &ShaderReflection,
        replicas: u32,
        device: &ash::Device,
    ) -> error::Result<Self> {
        let mut descriptors = PipelineDescriptors::new(replicas);
        descriptors.add_ubo_binding(reflection.ubo.as_ref());
        descriptors.add_texture_bindings(reflection.meta.texture_meta.values());

        let descriptor_set_layout = [descriptors.create_descriptor_set_layout(device)?];

        let pipeline_create_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layout);

        let pipeline_create_info = if let Some(push_constant) = &reflection.push_constant {
            let stage_mask = util::binding_stage_to_vulkan_stage(push_constant.stage_mask);
            let push_constant_range = [vk::PushConstantRange::builder()
                .stage_flags(stage_mask)
                .size(push_constant.size)
                .build()];
            pipeline_create_info
                .push_constant_ranges(&push_constant_range)
                .build()
        } else {
            pipeline_create_info.build()
        };

        let layout = unsafe { device.create_pipeline_layout(&pipeline_create_info, None)? };

        let pool_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(replicas)
            .pool_sizes(&descriptors.pool_sizes)
            .build();

        let pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

        let mut descriptor_sets = Vec::new();
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&descriptor_set_layout)
            .build();

        for _ in 0..replicas {
            let set = unsafe { device.allocate_descriptor_sets(&alloc_info)? };
            descriptor_sets.push(set)
        }

        let descriptor_sets: Vec<vk::DescriptorSet> =
            descriptor_sets.into_iter().flatten().collect();

        Ok(PipelineLayoutObjects {
            layout,
            descriptor_set_layout,
            descriptor_sets,
            pool,
        })
    }
}

pub struct VulkanShaderModule {
    shader: vk::ShaderModule,
    device: ash::Device,
}

impl VulkanShaderModule {
    pub fn new(
        device: &ash::Device,
        info: &vk::ShaderModuleCreateInfo,
    ) -> error::Result<VulkanShaderModule> {
        Ok(VulkanShaderModule {
            shader: unsafe { device.create_shader_module(info, None)? },
            device: device.clone(),
        })
    }
}

impl Drop for VulkanShaderModule {
    fn drop(&mut self) {
        unsafe { self.device.destroy_shader_module(self.shader, None) }
    }
}

pub struct VulkanGraphicsPipeline {
    pub layout: PipelineLayoutObjects,
    pub pipeline: vk::Pipeline,
}

impl VulkanGraphicsPipeline {
    pub fn new(
        device: &ash::Device,
        cache: &vk::PipelineCache,
        shader_assembly: &ShaderCompilerOutput<Vec<u32>>,
        reflection: &ShaderReflection,
        replicas: u32,
    ) -> error::Result<VulkanGraphicsPipeline> {
        // shader_vulkan 1927 (init_pipeline_layout)
        let pipeline_layout = PipelineLayoutObjects::new(reflection, replicas, device)?;

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
            .build();

        let vao_state = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: (2 * std::mem::size_of::<f32>()) as u32,
            },
        ];

        let input_binding = [vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(4 * std::mem::size_of::<f32>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()];

        let pipeline_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&input_binding)
            .vertex_attribute_descriptions(&vao_state)
            .build();

        let raster_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .depth_bias_enable(false)
            .line_width(1.0)
            .build();

        let attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::from_raw(0xf))
            .build()];

        let blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(&attachments)
            .build();

        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1)
            .build();

        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .stencil_test_enable(false)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(1.0)
            .max_depth_bounds(1.0)
            .build();

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .build();

        let states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&states)
            .build();

        let vertex_info = vk::ShaderModuleCreateInfo::builder()
            .code(shader_assembly.vertex.as_ref())
            .build();
        let fragment_info = vk::ShaderModuleCreateInfo::builder()
            .code(shader_assembly.fragment.as_ref())
            .build();

        let vertex_module = VulkanShaderModule::new(device, &vertex_info)?;
        let fragment_module = VulkanShaderModule::new(device, &fragment_info)?;

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .name(ENTRY_POINT)
                .module(vertex_module.shader)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .name(ENTRY_POINT)
                .module(fragment_module.shader)
                .build(),
        ];

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&pipeline_input_state)
            .input_assembly_state(&input_assembly)
            .rasterization_state(&raster_state)
            .color_blend_state(&blend_state)
            .multisample_state(&multisample_state)
            .viewport_state(&viewport_state)
            .depth_stencil_state(&depth_stencil_state)
            .dynamic_state(&dynamic_state)
            .layout(pipeline_layout.layout)
            .build();

        let pipeline = unsafe {
            // panic_safety: if this is successful this should return 1 pipelines.
            device
                .create_graphics_pipelines(*cache, &[pipeline_info], None)
                .map_err(|e| e.1)
                .unwrap()[0]
        };

        Ok(VulkanGraphicsPipeline {
            layout: pipeline_layout,
            pipeline,
        })
    }
}
