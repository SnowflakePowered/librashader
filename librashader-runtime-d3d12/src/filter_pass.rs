use rustc_hash::FxHashMap;
use librashader_common::Size;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::reflect::semantics::{MemberOffset, UniformBinding};
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::binding::TextureInput;
use librashader_runtime::uniforms::UniformStorage;
use crate::buffer::D3D12ConstantBuffer;
use crate::graphics_pipeline::D3D12GraphicsPipeline;
use crate::heap::{D3D12DescriptorHeap, ResourceWorkHeap, SamplerWorkHeap};
use crate::texture::InputTexture;

pub(crate) struct FilterPass {
    pub(crate) pipeline: D3D12GraphicsPipeline,
    pub(crate) reflection: ShaderReflection,
    pub(crate) config: ShaderPassConfig,
    pub(crate) uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub uniform_storage: UniformStorage,
    pub(crate) push_cbuffer: Option<D3D12ConstantBuffer>,
    pub(crate) ubo_cbuffer: Option<D3D12ConstantBuffer>,
    pub(crate) texture_heap: D3D12DescriptorHeap<ResourceWorkHeap>,
    pub(crate) sampler_heap: D3D12DescriptorHeap<SamplerWorkHeap>,
}

impl TextureInput for InputTexture {
    fn size(&self) -> Size<u32> {
        self.size
    }
}
