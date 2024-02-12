use icrate::Metal::MTLDevice;
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use rustc_hash::FxHashMap;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::reflect::semantics::{MemberOffset, UniformBinding};
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::uniforms::{NoUniformBinder, UniformStorage};
use crate::buffer::MetalBuffer;
use crate::graphics_pipeline::MetalGraphicsPipeline;

pub struct FilterPass {
    pub device: Id<ProtocolObject<dyn MTLDevice>>,
    pub reflection: ShaderReflection,
    pub(crate) uniform_storage:
    UniformStorage<NoUniformBinder, Option<()>, MetalBuffer, MetalBuffer>,
    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
    pub graphics_pipeline: MetalGraphicsPipeline,
}
