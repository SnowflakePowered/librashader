use crate::filter_chain::FilterCommon;
use crate::texture::{ExternalTexture, OwnedTexture};
use librashader_common::Size;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::cross::GlslangHlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{
    BindingStage, MemberOffset, TextureBinding, TextureSemantics, UniformBinding, UniformSemantic,
    VariableSemantics,
};
use librashader_reflect::reflect::ShaderReflection;
use rustc_hash::FxHashMap;
use std::error::Error;
use windows::core::ConstBuffer;
use windows::Win32::Graphics::Direct3D::ID3DBlob;
use windows::Win32::Graphics::Direct3D11::{ID3D11Buffer, ID3D11PixelShader, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11VertexShader, D3D11_MAP_WRITE_DISCARD, ID3D11InputLayout};

pub struct ConstantBufferBinding {
    pub binding: u32,
    pub size: u32,
    pub stage_mask: BindingStage,
    pub buffer: ID3D11Buffer,
}

pub struct ConstantBuffer {
    pub binding: Option<ConstantBufferBinding>,
    pub storage: Box<[u8]>,
}

impl ConstantBuffer {
    pub fn new(binding: Option<ConstantBufferBinding>) -> Self {
        let storage = vec![0u8; binding.as_ref().map(|c| c.size as usize).unwrap_or(0)].into_boxed_slice();
        Self {
            binding,
            storage
        }
    }
}

// slang_process.cpp 141
pub struct FilterPass {
    pub reflection: ShaderReflection,
    pub compiled: ShaderCompilerOutput<String, GlslangHlslContext>,
    pub vertex_shader: ID3D11VertexShader,
    pub vertex_layout: ID3D11InputLayout,
    pub pixel_shader: ID3D11PixelShader,

    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,

    pub uniform_buffer: ConstantBuffer,
    pub push_buffer: ConstantBuffer,
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
        texture_binding: &mut [Option<ID3D11ShaderResourceView>; 16],
        sampler_binding: &mut [Option<ID3D11SamplerState>; 16],
        binding: &TextureBinding,
        texture: &ExternalTexture,
    ) {
        texture_binding[binding.binding as usize] = Some(texture.srv.clone());
        // todo: make samplers for all wrapmode/filtermode combos.
        // sampler_binding[binding.binding as usize] = Some(texture.sampler.clone());
    }

    // framecount should be pre-modded
    fn build_semantics(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        mvp: &[f32],
        frame_count: u32,
        frame_direction: i32,
        fb_size: Size<u32>,
        // viewport: &Viewport,
        original: &ExternalTexture,
        source: &ExternalTexture,
    ) {
        let mut textures: [Option<ID3D11ShaderResourceView>; 16] = std::array::from_fn(|_| None);
        let mut samplers: [Option<ID3D11SamplerState>; 16] = std::array::from_fn(|_| None);

        // Bind MVP
        if let Some(offset) = self.uniform_bindings.get(&VariableSemantics::MVP.into()) {
            let mvp_size = mvp.len() * std::mem::size_of::<f32>();
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
            };
            FilterPass::build_mvp(&mut buffer[offset..][..mvp_size], mvp)
        }

        // bind OutputSize
        if let Some(offset) = self.uniform_bindings.get(&VariableSemantics::Output.into()) {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
            };

            FilterPass::build_vec4(&mut buffer[offset..][..16], fb_size)
        }

        // bind FinalViewportSize
        // if let Some(offset) = self
        //     .uniform_bindings
        //     .get(&VariableSemantics::FinalViewport.into())
        // {
        //     let (buffer, offset) = match offset {
        //         MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
        //         MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
        //     };
        //     FilterPass::build_vec4(
        //         &mut buffer[offset..][..16],
        //         viewport.output.size,
        //     )
        // }

        // bind FrameCount
        if let Some(offset) = self
            .uniform_bindings
            .get(&VariableSemantics::FrameCount.into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
            };
            FilterPass::build_uniform(&mut buffer[offset..][..4], frame_count)
        }

        // bind FrameDirection
        if let Some(offset) = self
            .uniform_bindings
            .get(&VariableSemantics::FrameDirection.into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
            };
            FilterPass::build_uniform(&mut buffer[offset..][..4], frame_direction)
        }

        // bind Original sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Original.semantics(0))
        {
            FilterPass::bind_texture(&mut textures, &mut samplers, binding, original);
        }
        //
        // bind OriginalSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::Original.semantics(0).into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
            };
            FilterPass::build_vec4(&mut buffer[offset..][..16], original.size);
        }

        // bind Source sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Source.semantics(0))
        {
            // eprintln!("setting source binding to {}", binding.binding);
            FilterPass::bind_texture(&mut textures, &mut samplers, binding, source);
        }

        // bind SourceSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::Source.semantics(0).into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
            };
            FilterPass::build_vec4(&mut buffer[offset..][..16], source.size);
        }

        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::OriginalHistory.semantics(0))
        {
            FilterPass::bind_texture(&mut textures, &mut samplers, binding, original);
        }

        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::OriginalHistory.semantics(0).into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
            };
            FilterPass::build_vec4(&mut buffer[offset..][..16], original.size);
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
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer.storage, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer.storage, *offset),
            };

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

            FilterPass::build_uniform(&mut buffer[offset..][..4], value)
        }

        // bind luts
        // for (index, lut) in &parent.luts {
        //     if let Some(binding) = self
        //         .reflection
        //         .meta
        //         .texture_meta
        //         .get(&TextureSemantics::User.semantics(*index))
        //     {
        //         FilterPass::bind_texture(binding, lut);
        //     }
        //
        //     if let Some((location, offset)) = self
        //         .uniform_bindings
        //         .get(&TextureSemantics::User.semantics(*index).into())
        //     {
        //         let (buffer, offset) = match offset {
        //             MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
        //             MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
        //         };
        //         FilterPass::build_vec4(
        //             location.location(),
        //             &mut buffer[offset..][..16],
        //             lut.image.size,
        //         );
        //     }
        // }
    }

    pub fn draw(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        frame_count: u32,
        frame_direction: i32,
    ) -> std::result::Result<(), Box<dyn Error>> {
        Ok(())
    }
}
