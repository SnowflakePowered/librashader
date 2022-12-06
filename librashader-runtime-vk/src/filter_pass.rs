use rustc_hash::FxHashMap;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{BindingStage, MemberOffset, TextureBinding, UboReflection, UniformBinding};
use librashader_runtime::uniforms::UniformStorage;
use ash::vk;

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
            let mut ubo_mask = vk::ShaderStageFlags::default();
            if ubo_meta.stage_mask.contains(BindingStage::VERTEX) {
                ubo_mask |= vk::ShaderStageFlags::VERTEX;
            }
            if ubo_meta.stage_mask.contains(BindingStage::FRAGMENT) {
                ubo_mask |= vk::ShaderStageFlags::FRAGMENT;
            }

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
}