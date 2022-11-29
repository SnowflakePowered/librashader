use crate::filter_chain::FilterCommon;
use crate::texture::{Texture, OwnedTexture};
use librashader_common::Size;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::cross::GlslangHlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{BindingStage, MAX_BINDINGS_COUNT, MemberOffset, TextureBinding, TextureSemantics, UniformBinding, UniformSemantic, VariableSemantics};
use librashader_reflect::reflect::ShaderReflection;
use rustc_hash::FxHashMap;
use std::error::Error;
use windows::core::ConstBuffer;
use windows::Win32::Graphics::Direct3D::ID3DBlob;
use windows::Win32::Graphics::Direct3D11::{ID3D11Buffer, ID3D11PixelShader, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11VertexShader, D3D11_MAP_WRITE_DISCARD, ID3D11InputLayout};
use librashader_runtime::uniforms::UniformStorage;
use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;

pub struct ConstantBufferBinding {
    pub binding: u32,
    pub size: u32,
    pub stage_mask: BindingStage,
    pub buffer: ID3D11Buffer,
}

// slang_process.cpp 141
pub struct FilterPass {
    pub reflection: ShaderReflection,
    pub compiled: ShaderCompilerOutput<String, GlslangHlslContext>,
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
// slang_process.cpp 229
impl FilterPass {
    fn build_mvp(buffer: &mut [u8], mvp: &[f32]) {
        let mvp = bytemuck::cast_slice(mvp);
        buffer.copy_from_slice(mvp);
    }

    #[inline(always)]
    fn build_uniform<T>(buffer: &mut [u8], value: T)
    where
        T: Copy,
        T: bytemuck::Pod,
    {
        let buffer = bytemuck::cast_slice_mut(buffer);
        buffer[0] = value;
    }

    fn build_vec4(buffer: &mut [u8], size: impl Into<[f32; 4]>) {
        let vec4 = size.into();
        let vec4 = bytemuck::cast_slice(&vec4);
        buffer.copy_from_slice(vec4);
    }

    fn bind_texture(
        samplers: &SamplerSet,
        texture_binding: &mut [Option<ID3D11ShaderResourceView>; 16],
        sampler_binding: &mut [Option<ID3D11SamplerState>; 16],
        binding: &TextureBinding,
        texture: &Texture,
    ) {
        texture_binding[binding.binding as usize] = Some(texture.view.handle.clone());
        sampler_binding[binding.binding as usize] = Some(samplers.get(texture.wrap_mode, texture.filter).clone());
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
        original: &Texture,
        source: &Texture,
    ) -> ([Option<ID3D11ShaderResourceView>; 16], [Option<ID3D11SamplerState>; 16]){
        let mut textures: [Option<ID3D11ShaderResourceView>; 16] = std::array::from_fn(|_| None);
        let mut samplers: [Option<ID3D11SamplerState>; 16] = std::array::from_fn(|_| None);

        // Bind MVP
        if let Some(offset) = self.uniform_bindings.get(&VariableSemantics::MVP.into()) {
            self.uniform_storage.bind_mat4(*offset, mvp, None);
        }

        // bind OutputSize
        if let Some(offset) = self.uniform_bindings.get(&VariableSemantics::Output.into()) {
            self.uniform_storage.bind_vec4(*offset, fb_size, None);
        }

        // bind FinalViewportSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&VariableSemantics::FinalViewport.into())
        {
            self.uniform_storage.bind_vec4(*offset, viewport_size, None);
        }

        // bind FrameCount
        if let Some(offset) = self
            .uniform_bindings
            .get(&VariableSemantics::FrameCount.into())
        {
            self.uniform_storage.bind_scalar(*offset, frame_count, None);
        }

        // bind FrameDirection
        if let Some(offset) = self
            .uniform_bindings
            .get(&VariableSemantics::FrameDirection.into())
        {
            self.uniform_storage.bind_scalar(*offset, frame_direction, None);
        }

        // bind Original sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Original.semantics(0))
        {
            FilterPass::bind_texture(&parent.samplers, &mut textures, &mut samplers, binding, original);
        }
        //
        // bind OriginalSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::Original.semantics(0).into())
        {
            self.uniform_storage.bind_vec4(*offset, original.view.size, None);
        }

        // bind Source sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Source.semantics(0))
        {
            // eprintln!("setting source binding to {}", binding.binding);
            FilterPass::bind_texture(&parent.samplers, &mut textures, &mut samplers, binding, source);
        }

        // bind SourceSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::Source.semantics(0).into())
        {
            self.uniform_storage.bind_vec4(*offset, source.view.size, None);
        }

        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::OriginalHistory.semantics(0))
        {
            FilterPass::bind_texture(&parent.samplers, &mut textures, &mut samplers, binding, original);
        }

        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::OriginalHistory.semantics(0).into())
        {
            self.uniform_storage.bind_vec4(*offset, original.view.size, None);
        }

        // for (index, output) in parent.history_textures.iter().enumerate() {
        //     // if let Some(binding) = self
        //     //     .reflection
        //     //     .meta
        //     //     .texture_meta
        //     //     .get(&TextureSemantics::OriginalHistory.semantics(index + 1))
        //     // {
        //     //     FilterPass::bind_texture(binding, output);
        //     // }
        //
        //     if let Some((location, offset)) = self.uniform_bindings.get(
        //         &TextureSemantics::OriginalHistory
        //             .semantics(index + 1)
        //             .into(),
        //     ) {
        //         let (buffer, offset) = match offset {
        //             MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
        //             MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
        //         };
        //         FilterPass::build_vec4(
        //             location.location(),
        //             &mut buffer[offset..][..16],
        //             output.image.size,
        //         );
        //     }
        // }

        // PassOutput
        // for (index, output) in parent.output_textures.iter().enumerate() {
        //     if let Some(binding) = self
        //         .reflection
        //         .meta
        //         .texture_meta
        //         .get(&TextureSemantics::PassOutput.semantics(index))
        //     {
        //         FilterPass::bind_texture(binding, output);
        //     }
        //
        //     if let Some(offset) = self
        //         .uniform_bindings
        //         .get(&TextureSemantics::PassOutput.semantics(index).into())
        //     {
        //         let (buffer, offset) = match offset {
        //             MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
        //             MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
        //         };
        //         FilterPass::build_uniform(
        //             &mut buffer[offset..][..16],
        //             output.image.size,
        //         );
        //     }
        // }

        // PassFeedback
        // for (index, feedback) in parent.feedback_textures.iter().enumerate() {
        //     // if let Some(binding) = self
        //     //     .reflection
        //     //     .meta
        //     //     .texture_meta
        //     //     .get(&TextureSemantics::PassFeedback.semantics(index))
        //     // {
        //     //     if feedback.image.handle == 0 {
        //     //         eprintln!("[WARNING] trying to bind PassFeedback: {index} which has texture 0 to slot {} in pass {pass_index}", binding.binding)
        //     //     }
        //     //     FilterPass::bind_texture(binding, feedback);
        //     // }
        //
        //     if let Some(offset) = self
        //         .uniform_bindings
        //         .get(&TextureSemantics::PassFeedback.semantics(index).into())
        //     {
        //         let (buffer, offset) = match offset {
        //             MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
        //             MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
        //         };
        //         FilterPass::build_uniform(
        //             &mut buffer[offset..][..16],
        //             feedback.image.size,
        //         );
        //     }
        // }

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

            // todo: cache parameters.
            // presets override params
            let default = self
                .source
                .parameters
                .iter()
                .find(|&p| p.id == id)
                .map(|f| f.initial)
                .unwrap_or(0f32);

            let value = parent
                .preset
                .parameters
                .iter()
                .find(|&p| p.name == id)
                .map(|p| p.value)
                .unwrap_or(default);

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
                FilterPass::bind_texture(&parent.samplers, &mut textures, &mut samplers, binding, &lut.image);
            }

            if let Some(offset) = self
                .uniform_bindings
                .get(&TextureSemantics::User.semantics(*index).into())
            {
                self.uniform_storage.bind_vec4(*offset, lut.image.view.size, None);
            }
        }

        (textures, samplers)
    }

    pub fn draw(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        frame_count: u32,
        frame_direction: i32,
        viewport: &Size<u32>,
        original: &Texture,
        source: &Texture,
        output: RenderTarget,
    ) -> std::result::Result<(), Box<dyn Error>> {
        let device = &parent.d3d11.device;
        let context = &parent.d3d11.device_context;
        unsafe {
            context.IASetInputLayout(&self.vertex_layout);
            context.VSSetShader(&self.vertex_shader, None);
            context.PSSetShader(&self.pixel_shader, None);
        }

        let (textures, samplers) = self.build_semantics(pass_index, parent, output.mvp, frame_count, frame_direction,
                             output.output.size, *viewport, original, source);



        if let Some(ubo) = &self.uniform_buffer {
            // upload uniforms
            unsafe {
                let map = context.Map(&ubo.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0)?;
                std::ptr::copy_nonoverlapping(self.uniform_storage.ubo.as_ptr(), map.pData.cast(), ubo.size as usize);
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
                std::ptr::copy_nonoverlapping(self.uniform_storage.push.as_ptr(), map.pData.cast(), push.size as usize);
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
            context.RSSetViewports(Some(&[output.output.viewport.clone()]))
        }

        unsafe {
            // must be under primitive topology trianglestrip with quad
            context.Draw(4, 0);
        }
        Ok(())
    }
}
