use crate::graphics_pipeline::WgpuGraphicsPipeline;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::wgsl::NagaWgslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{MemberOffset, UniformBinding};
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::uniforms::{NoUniformBinder, UniformStorage};
use rustc_hash::FxHashMap;
use std::sync::Arc;

pub struct FilterPass {
    pub reflection: ShaderReflection,
    pub(crate) compiled: ShaderCompilerOutput<String, NagaWgslContext>,
    pub(crate) uniform_storage: UniformStorage,
    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
    pub graphics_pipeline: WgpuGraphicsPipeline,
    // pub ubo_ring: VkUboRing,
    // pub frames_in_flight: u32,
}
