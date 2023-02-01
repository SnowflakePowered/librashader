use rustc_hash::FxHashMap;
use windows::Win32::Graphics::Direct3D11::ID3D11Device;
use windows::Win32::Graphics::Direct3D12::ID3D12Device;
use librashader_common::{ImageFormat, Size};
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::reflect::semantics::{MemberOffset, TextureBinding, UniformBinding};
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::binding::{BindSemantics, TextureInput};
use librashader_runtime::uniforms::UniformStorage;
use crate::buffer::D3D12ConstantBuffer;
use crate::filter_chain::FilterCommon;
use crate::graphics_pipeline::D3D12GraphicsPipeline;
use crate::heap::{D3D12DescriptorHeap, D3D12DescriptorHeapSlot, ResourceWorkHeap, SamplerWorkHeap};
use crate::samplers::SamplerSet;
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
    pub source: ShaderSource,

}

impl TextureInput for InputTexture {
    fn size(&self) -> Size<u32> {
        self.size
    }
}
//
impl BindSemantics for FilterPass {
    type InputTexture = InputTexture;
    type SamplerSet = SamplerSet;
    type DescriptorSet<'a> =
    (
        &'a mut [D3D12DescriptorHeapSlot<ResourceWorkHeap>; 16],
        &'a mut [D3D12DescriptorHeapSlot<SamplerWorkHeap>; 16],
    );
    type DeviceContext = ();
    type UniformOffset = MemberOffset;

    fn bind_texture<'a>(
        descriptors: &mut Self::DescriptorSet<'a>,
        samplers: &Self::SamplerSet,
        binding: &TextureBinding,
        texture: &Self::InputTexture,
        _device: &Self::DeviceContext,
    ) {
        let (texture_binding,
            sampler_binding) = descriptors;

        unsafe {
            texture_binding[binding.binding as usize]
                .copy_descriptor(*texture.descriptor.as_ref());
            sampler_binding[binding.binding as usize]
                .copy_descriptor(*samplers.get(texture.wrap_mode, texture.filter).as_ref())
        }
    }
}

impl FilterPass {
    pub fn get_format(&self) -> ImageFormat {
        let fb_format = self.source.format;
        if let Some(format) = self.config.get_format_override() {
            format
        } else if fb_format == ImageFormat::Unknown {
            ImageFormat::R8G8B8A8Unorm
        } else {
            fb_format
        }
    }

    // framecount should be pre-modded
    fn build_semantics<'a>(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        mvp: &[f32; 16],
        frame_count: u32,
        frame_direction: i32,
        fb_size: Size<u32>,
        viewport_size: Size<u32>,
        mut descriptors: (
            &'a mut [D3D12DescriptorHeapSlot<ResourceWorkHeap>; 16],
            &'a mut [D3D12DescriptorHeapSlot<SamplerWorkHeap>; 16],
        ),
        original: &InputTexture,
        source: &InputTexture,
    ) {
        Self::bind_semantics(
            &(),
            &parent.samplers,
            &mut self.uniform_storage,
            &mut descriptors,
            mvp,
            frame_count,
            frame_direction,
            fb_size,
            viewport_size,
            original,
            source,
            &self.uniform_bindings,
            &self.reflection.meta.texture_meta,
            parent.output_textures[0..pass_index]
                .iter()
                .map(|o| o.as_ref()),
            parent.feedback_textures.iter().map(|o| o.as_ref()),
            parent.history_textures.iter().map(|o| o.as_ref()),
            parent.luts.iter().map(|(u, i)| (*u, i.as_ref())),
            &self.source.parameters,
            &parent.config.parameters,
        );
    }
}