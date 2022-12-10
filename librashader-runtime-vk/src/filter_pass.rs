use rustc_hash::FxHashMap;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{MemberOffset, UniformBinding};
use librashader_runtime::uniforms::UniformStorage;
use crate::vulkan_state::VulkanGraphicsPipeline;

pub struct FilterPass {
    pub(crate) compiled: ShaderCompilerOutput<Vec<u32>>,
    pub(crate) uniform_storage: UniformStorage,
    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
    pub graphics_pipeline: VulkanGraphicsPipeline,
}
