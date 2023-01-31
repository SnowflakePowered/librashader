use rustc_hash::FxHashMap;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::reflect::semantics::{MemberOffset, UniformBinding};
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::uniforms::UniformStorage;
use crate::buffer::D3D12ConstantBuffer;
use crate::graphics_pipeline::D3D12GraphicsPipeline;

pub(crate) struct FilterPass {
    pub(crate) pipeline: D3D12GraphicsPipeline,
    pub(crate) reflection: ShaderReflection,
    pub(crate) config: ShaderPassConfig,
    pub(crate) uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub uniform_storage: UniformStorage,
    pub(crate) push_cbuffer: Option<D3D12ConstantBuffer>,
    pub(crate) ubo_cbuffer: Option<D3D12ConstantBuffer>,
}

