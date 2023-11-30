use std::sync::Arc;
use rustc_hash::FxHashMap;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{MemberOffset, UniformBinding};
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::uniforms::{NoUniformBinder, UniformStorage};

pub struct FilterPass {
    pub reflection: ShaderReflection,
    pub(crate) compiled: ShaderCompilerOutput<Vec<u32>>,
    // pub(crate) uniform_storage: UniformStorage<NoUniformBinder, Option<()>, RawVulkanBuffer>,
    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
    // pub graphics_pipeline: VulkanGraphicsPipeline,
    // pub ubo_ring: VkUboRing,
    // pub frames_in_flight: u32,
}
