use crate::filter_chain::FilterCommon;
use crate::texture::InputTexture;
use librashader_common::{ImageFormat, Size, Viewport};
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::cross::CrossHlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{
    BindingStage, MemberOffset, TextureBinding, TextureSemantics, UniformBinding, UniqueSemantics,
};
use librashader_reflect::reflect::ShaderReflection;
use rustc_hash::FxHashMap;

use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11InputLayout, ID3D11PixelShader, ID3D11SamplerState,
    ID3D11ShaderResourceView, ID3D11VertexShader, D3D11_MAP_WRITE_DISCARD,
};

use crate::{D3D11OutputView, error};
use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;
use librashader_runtime::uniforms::{UniformStorage, UniformStorageAccess};

pub struct ConstantBufferBinding {
    pub binding: u32,
    pub size: u32,
    pub stage_mask: BindingStage,
    pub buffer: ID3D11Buffer,
}

// slang_process.cpp 141
pub struct FilterPass {
    pub reflection: ShaderReflection,
    pub compiled: ShaderCompilerOutput<String, CrossHlslContext>,
    pub vertex_shader: ID3D11VertexShader,
    pub vertex_layout: ID3D11InputLayout,
    pub pixel_shader: ID3D11PixelShader,

    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,

    pub uniform_storage: UniformStorage,
    pub uniform_buffer: Option<ConstantBufferBinding>,
    pub push_buffer: Option<ConstantBufferBinding>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
}

// https://doc.rust-lang.org/nightly/core/array/fn.from_fn.html is not ~const :(
const NULL_TEXTURES: &[Option<ID3D11ShaderResourceView>; 16] = &[
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
];

// slang_process.cpp 229
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

    fn bind_texture(
        samplers: &SamplerSet,
        texture_binding: &mut [Option<ID3D11ShaderResourceView>; 16],
        sampler_binding: &mut [Option<ID3D11SamplerState>; 16],
        binding: &TextureBinding,
        texture: &InputTexture,
    ) {
        texture_binding[binding.binding as usize] = Some(texture.view.handle.clone());
        sampler_binding[binding.binding as usize] =
            Some(samplers.get(texture.wrap_mode, texture.filter).clone());
    }

    // framecount should be pre-modded
    fn build_semantics(
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
    ) -> (
        [Option<ID3D11ShaderResourceView>; 16],
        [Option<ID3D11SamplerState>; 16],
    ) {
        let mut textures: [Option<ID3D11ShaderResourceView>; 16] = std::array::from_fn(|_| None);
        let mut samplers: [Option<ID3D11SamplerState>; 16] = std::array::from_fn(|_| None);

        // Bind MVP
        if let Some(offset) = self.uniform_bindings.get(&UniqueSemantics::MVP.into()) {
            self.uniform_storage.bind_mat4(*offset, mvp, None);
        }

        // bind OutputSize
        if let Some(offset) = self.uniform_bindings.get(&UniqueSemantics::Output.into()) {
            self.uniform_storage.bind_vec4(*offset, fb_size, None);
        }

        // bind FinalViewportSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&UniqueSemantics::FinalViewport.into())
        {
            self.uniform_storage.bind_vec4(*offset, viewport_size, None);
        }

        // bind FrameCount
        if let Some(offset) = self
            .uniform_bindings
            .get(&UniqueSemantics::FrameCount.into())
        {
            self.uniform_storage.bind_scalar(*offset, frame_count, None);
        }

        // bind FrameDirection
        if let Some(offset) = self
            .uniform_bindings
            .get(&UniqueSemantics::FrameDirection.into())
        {
            self.uniform_storage
                .bind_scalar(*offset, frame_direction, None);
        }

        // bind Original sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Original.semantics(0))
        {
            FilterPass::bind_texture(
                &parent.samplers,
                &mut textures,
                &mut samplers,
                binding,
                original,
            );
        }
        //
        // bind OriginalSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::Original.semantics(0).into())
        {
            self.uniform_storage
                .bind_vec4(*offset, original.view.size, None);
        }

        // bind Source sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Source.semantics(0))
        {
            // eprintln!("setting source binding to {}", binding.binding);
            FilterPass::bind_texture(
                &parent.samplers,
                &mut textures,
                &mut samplers,
                binding,
                source,
            );
        }

        // bind SourceSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::Source.semantics(0).into())
        {
            self.uniform_storage
                .bind_vec4(*offset, source.view.size, None);
        }

        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::OriginalHistory.semantics(0))
        {
            FilterPass::bind_texture(
                &parent.samplers,
                &mut textures,
                &mut samplers,
                binding,
                original,
            );
        }

        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::OriginalHistory.semantics(0).into())
        {
            self.uniform_storage
                .bind_vec4(*offset, original.view.size, None);
        }

        for (index, output) in parent.history_textures.iter().enumerate() {
            let Some(output) = output else {
                // eprintln!("no history");
                continue;
            };
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::OriginalHistory.semantics(index + 1))
            {
                FilterPass::bind_texture(
                    &parent.samplers,
                    &mut textures,
                    &mut samplers,
                    binding,
                    output,
                );
            }

            if let Some(offset) = self.uniform_bindings.get(
                &TextureSemantics::OriginalHistory
                    .semantics(index + 1)
                    .into(),
            ) {
                self.uniform_storage
                    .bind_vec4(*offset, output.view.size, None);
            }
        }

        // PassOutput
        for (index, output) in parent.output_textures[0..pass_index].iter().enumerate() {
            let Some(output) = output else {
                continue;
            };
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::PassOutput.semantics(index))
            {
                FilterPass::bind_texture(
                    &parent.samplers,
                    &mut textures,
                    &mut samplers,
                    binding,
                    output,
                );
            }

            if let Some(offset) = self
                .uniform_bindings
                .get(&TextureSemantics::PassOutput.semantics(index).into())
            {
                self.uniform_storage
                    .bind_vec4(*offset, output.view.size, None);
            }
        }

        // PassFeedback
        for (index, feedback) in parent.feedback_textures.iter().enumerate() {
            let Some(feedback) = feedback else {
                // eprintln!("no passfeedback {index}");
                continue;
            };
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::PassFeedback.semantics(index))
            {
                FilterPass::bind_texture(
                    &parent.samplers,
                    &mut textures,
                    &mut samplers,
                    binding,
                    feedback,
                );
            }

            if let Some(offset) = self
                .uniform_bindings
                .get(&TextureSemantics::PassFeedback.semantics(index).into())
            {
                self.uniform_storage
                    .bind_vec4(*offset, feedback.view.size, None);
            }
        }

        // bind float parameters
        for (id, offset) in
            self.uniform_bindings
                .iter()
                .filter_map(|(binding, value)| match binding {
                    UniformBinding::Parameter(id) => Some((id, value)),
                    _ => None,
                })
        {
            let id = id.as_str();

            let default = self
                .source
                .parameters
                .iter()
                .find(|&p| p.id == id)
                .map(|f| f.initial)
                .unwrap_or(0f32);

            let value = *parent.config.parameters.get(id).unwrap_or(&default);

            self.uniform_storage.bind_scalar(*offset, value, None);
        }

        // bind luts
        for (index, lut) in &parent.luts {
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::User.semantics(*index))
            {
                FilterPass::bind_texture(
                    &parent.samplers,
                    &mut textures,
                    &mut samplers,
                    binding,
                    &lut.image,
                );
            }

            if let Some(offset) = self
                .uniform_bindings
                .get(&TextureSemantics::User.semantics(*index).into())
            {
                self.uniform_storage
                    .bind_vec4(*offset, lut.image.view.size, None);
            }
        }

        (textures, samplers)
    }

    pub(crate) fn draw(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        frame_count: u32,
        frame_direction: i32,
        viewport: &Viewport<D3D11OutputView>,
        original: &InputTexture,
        source: &InputTexture,
        output: RenderTarget,
    ) -> error::Result<()> {
        let _device = &parent.d3d11.device;
        let context = &parent.d3d11.current_context;

        if self.config.mipmap_input && !parent.disable_mipmaps {
            unsafe {
                context.GenerateMips(&source.view.handle);
            }
        }
        unsafe {
            context.IASetInputLayout(&self.vertex_layout);
            context.VSSetShader(&self.vertex_shader, None);
            context.PSSetShader(&self.pixel_shader, None);
        }

        let (textures, samplers) = self.build_semantics(
            pass_index,
            parent,
            output.mvp,
            frame_count,
            frame_direction,
            output.output.size,
            viewport.output.size,
            original,
            source,
        );

        if let Some(ubo) = &self.uniform_buffer {
            // upload uniforms
            unsafe {
                let map = context.Map(&ubo.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0)?;
                std::ptr::copy_nonoverlapping(
                    self.uniform_storage.ubo_pointer(),
                    map.pData.cast(),
                    ubo.size as usize,
                );
                context.Unmap(&ubo.buffer, 0);
            }

            if ubo.stage_mask.contains(BindingStage::VERTEX) {
                unsafe {
                    context.VSSetConstantBuffers(ubo.binding, Some(&[Some(ubo.buffer.clone())]))
                }
            }
            if ubo.stage_mask.contains(BindingStage::FRAGMENT) {
                unsafe {
                    context.PSSetConstantBuffers(ubo.binding, Some(&[Some(ubo.buffer.clone())]))
                }
            }
        }

        if let Some(push) = &self.push_buffer {
            // upload push constants
            unsafe {
                let map = context.Map(&push.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0)?;
                std::ptr::copy_nonoverlapping(
                    self.uniform_storage.push_pointer(),
                    map.pData.cast(),
                    push.size as usize,
                );
                context.Unmap(&push.buffer, 0);
            }

            if push.stage_mask.contains(BindingStage::VERTEX) {
                unsafe {
                    context.VSSetConstantBuffers(push.binding, Some(&[Some(push.buffer.clone())]))
                }
            }
            if push.stage_mask.contains(BindingStage::FRAGMENT) {
                unsafe {
                    context.PSSetConstantBuffers(push.binding, Some(&[Some(push.buffer.clone())]))
                }
            }
        }

        unsafe {
            // reset RTVs
            context.OMSetRenderTargets(None, None);
        }

        unsafe {
            context.PSSetShaderResources(0, Some(&textures));
            context.PSSetSamplers(0, Some(&samplers));

            context.OMSetRenderTargets(Some(&[Some(output.output.rtv.clone())]), None);
            context.RSSetViewports(Some(&[output.output.viewport]))
        }

        unsafe {
            // must be under primitive topology trianglestrip with quad
            context.Draw(4, 0);
        }

        unsafe {
            // unbind resources.
            context.PSSetShaderResources(0, Some(NULL_TEXTURES));
            context.OMSetRenderTargets(None, None);
        }
        Ok(())
    }
}
