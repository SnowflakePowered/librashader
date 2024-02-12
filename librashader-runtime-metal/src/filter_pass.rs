use std::sync::Arc;
use icrate::Metal::{MTLDevice, MTLRenderCommandEncoder, MTLTexture};
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use rustc_hash::FxHashMap;
use librashader_common::Size;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::reflect::semantics::{MemberOffset, TextureBinding, UniformBinding};
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::binding::{BindSemantics, TextureInput};
use librashader_runtime::uniforms::{NoUniformBinder, UniformStorage};
use crate::buffer::MetalBuffer;
use crate::filter_chain::FilterCommon;
use crate::graphics_pipeline::MetalGraphicsPipeline;
use crate::samplers::SamplerSet;
use crate::texture::InputTexture;

impl TextureInput for InputTexture {
    fn size(&self) -> Size<u32> {
        let height = self.texture.height();
        let width = self.texture.width();
        Size {
            height: height as u32,
            width: width as u32
        }
    }
}


impl BindSemantics<NoUniformBinder, Option<()>, MetalBuffer, MetalBuffer> for FilterPass {
    type InputTexture = InputTexture;
    type SamplerSet = SamplerSet;
    type DescriptorSet<'a> = &'a ProtocolObject<dyn MTLRenderCommandEncoder>;
    type DeviceContext = ();
    type UniformOffset = MemberOffset;

    #[inline(always)]
    fn bind_texture<'a>(
        renderpass: &mut Self::DescriptorSet<'a>,
        samplers: &Self::SamplerSet,
        binding: &TextureBinding,
        texture: &Self::InputTexture,
        _device: &Self::DeviceContext,
    ) {
        let sampler = samplers.get(texture.wrap_mode, texture.filter_mode, texture.mip_filter);

        unsafe {
            renderpass
                .setFragmentTexture_atIndex(Some(&texture.texture), binding.binding as usize);
            renderpass
                .setFragmentTexture_atIndex(Some(&texture.texture), binding.binding as usize);
            renderpass
                .setFragmentSamplerState_atIndex(Some(sampler), binding.binding as usize);
        }
    }
}

pub struct FilterPass {
    pub reflection: ShaderReflection,
    pub(crate) uniform_storage: UniformStorage<NoUniformBinder, Option<()>, MetalBuffer, MetalBuffer>,
    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
    pub graphics_pipeline: MetalGraphicsPipeline,
}

impl FilterPass {
    fn build_semantics<'a>(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        mvp: &[f32; 16],
        frame_count: u32,
        frame_direction: i32,
        fb_size: Size<u32>,
        viewport_size: Size<u32>,
        original: &InputTexture,
        source: &InputTexture,
        mut renderpass: &ProtocolObject<dyn MTLRenderCommandEncoder>
    ) {
        Self::bind_semantics(
            &(),
            &parent.samplers,
            &mut self.uniform_storage,
            &mut renderpass,
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

        // flush to buffers
        self.uniform_storage.inner_ubo().flush();
        self.uniform_storage.inner_push().flush();
    }
}