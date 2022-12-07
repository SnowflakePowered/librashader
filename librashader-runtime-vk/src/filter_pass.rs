use rustc_hash::FxHashMap;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{BindingStage, MemberOffset, TextureBinding, UboReflection, UniformBinding};
use librashader_runtime::uniforms::UniformStorage;
use ash::vk;
use librashader_reflect::reflect::ShaderReflection;
use crate::{error, util};

pub struct FilterPass {
    pub(crate) compiled: ShaderCompilerOutput<Vec<u32>>,
    pub(crate) uniform_storage: UniformStorage,
    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
}

pub struct PipelineDescriptors {
    pub replicas: u32,
    pub layout_bindings: Vec<vk::DescriptorSetLayoutBinding>,
    pub pool_sizes: Vec<vk::DescriptorPoolSize>
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
            let mut ubo_mask = util::binding_stage_to_vulkan_stage(ubo_meta.stage_mask);

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
        let mut texture_mask = vk::ShaderStageFlags::FRAGMENT;
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

    pub fn binding_count(&self) -> usize {
        self.layout_bindings.len()
    }

    pub fn bindings(&self) -> &[vk::DescriptorSetLayoutBinding] {
        self.layout_bindings.as_ref()
    }

    pub fn create_descriptor_set_layout(&self, device: &ash::Device) -> error::Result<vk::DescriptorSetLayout> {
        unsafe {
            let layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder()
                    .bindings(self.bindings())
                    .build(),
                None)?;
            Ok(layout)
        }
    }
}

pub struct PipelineObjects {
    pub layout: vk::PipelineLayout,
    pub pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub descriptor_set_layout: [vk::DescriptorSetLayout;1],
}

impl PipelineObjects {
    pub fn new(reflection: &ShaderReflection, replicas: u32, device: &ash::Device) -> error::Result<Self> {
        let mut descriptors = PipelineDescriptors::new(replicas);
        descriptors.add_ubo_binding(reflection.ubo.as_ref());
        descriptors.add_texture_bindings(reflection.meta.texture_meta.values());

        let mut descriptor_set_layout = [descriptors.create_descriptor_set_layout(device)?];

        let mut pipeline_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&descriptor_set_layout);

        let pipeline_create_info = if let Some(push_constant) = &reflection.push_constant {
            let mut stage_mask = util::binding_stage_to_vulkan_stage(push_constant.stage_mask);
            let push_constant_range = [
                vk::PushConstantRange::builder()
                    .stage_flags(stage_mask)
                    .size(push_constant.size)
                    .build()
            ];
            pipeline_create_info.push_constant_ranges(&push_constant_range).build()
        } else {
            pipeline_create_info.build()
        };

        let layout = unsafe {
            device.create_pipeline_layout(&pipeline_create_info, None)?
        };

        let pool = unsafe {
            device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo::builder()
                .max_sets(replicas)
                .pool_sizes(&descriptors.pool_sizes)
                .build(), None)?
        };

        let mut descriptor_sets = Vec::new();
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&descriptor_set_layout)
            .build();

        for _ in 0..replicas {
            unsafe {
                descriptor_sets.push(device.allocate_descriptor_sets(&alloc_info)?)
            }
        }

        let descriptor_sets: Vec<vk::DescriptorSet> = descriptor_sets.into_iter().flatten().collect();

        return Ok(PipelineObjects {
            layout,
            descriptor_set_layout,
            descriptor_sets,
            pool,
        })
    }
}
