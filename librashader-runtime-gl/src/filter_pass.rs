use gl::types::{GLint, GLsizei, GLsizeiptr, GLuint};
use librashader_reflect::back::cross::GlslangGlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::ShaderReflection;

use librashader_common::{ShaderFormat, Size};
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::reflect::semantics::{BindingStage, MemberOffset, TextureBinding, TextureSemantics, UniformBinding, VariableSemantics};
use rustc_hash::FxHashMap;

use crate::binding::{UniformLocation, VariableLocation};
use crate::filter_chain::FilterCommon;
use crate::framebuffer::Viewport;
use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;
use crate::texture::Texture;
use crate::util::{InlineRingBuffer, RingBuffer};

pub struct FilterPass {
    pub reflection: ShaderReflection,
    pub compiled: ShaderCompilerOutput<String, GlslangGlslContext>,
    pub program: GLuint,
    pub ubo_location: UniformLocation<GLuint>,
    pub ubo_ring: Option<InlineRingBuffer<GLuint, 16>>,
    pub uniform_buffer: Box<[u8]>,
    pub push_buffer: Box<[u8]>,
    pub uniform_bindings: FxHashMap<UniformBinding, (VariableLocation, MemberOffset)>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
}

impl FilterPass {
    fn build_mat4(location: UniformLocation<GLint>, buffer: &mut [u8], mvp: &[f32; 16]) {
        if location.is_valid(BindingStage::VERTEX | BindingStage::FRAGMENT) {
            unsafe {
                if location.is_valid(BindingStage::VERTEX) {
                    gl::UniformMatrix4fv(location.vertex, 1, gl::FALSE, mvp.as_ptr());
                }
                if location.is_valid(BindingStage::FRAGMENT) {
                    gl::UniformMatrix4fv(location.fragment, 1, gl::FALSE, mvp.as_ptr());
                }
            }
        } else {
            let mvp = bytemuck::cast_slice(mvp);
            buffer.copy_from_slice(mvp);
        }
    }

    fn build_vec4(location: UniformLocation<GLint>, buffer: &mut [u8], size: impl Into<[f32; 4]>) {
        let vec4 = size.into();
        if location.is_valid(BindingStage::VERTEX | BindingStage::FRAGMENT) {
            unsafe {
                if location.is_valid(BindingStage::VERTEX) {
                    gl::Uniform4fv(location.vertex, 1, vec4.as_ptr());
                }
                if location.is_valid(BindingStage::FRAGMENT) {
                    gl::Uniform4fv(location.fragment, 1, vec4.as_ptr());
                }
            }
        } else {
            let vec4 = bytemuck::cast_slice(&vec4);
            buffer.copy_from_slice(vec4);
        }
    }

    #[inline(always)]
    fn build_uniform<T>(
        location: UniformLocation<GLint>,
        buffer: &mut [u8],
        value: T,
        glfn: unsafe fn(GLint, T) -> (),
    ) where
        T: Copy,
        T: bytemuck::Pod,
    {
        if location.is_valid(BindingStage::VERTEX | BindingStage::FRAGMENT) {
            unsafe {
                if location.is_valid(BindingStage::VERTEX) {
                    glfn(location.vertex, value);
                }
                if location.is_valid(BindingStage::FRAGMENT) {
                    glfn(location.fragment, value);
                }
            }
        } else {
            let buffer = bytemuck::cast_slice_mut(buffer);
            buffer[0] = value;
        }
    }

    fn build_uint(location: UniformLocation<GLint>, buffer: &mut [u8], value: u32) {
        Self::build_uniform(location, buffer, value, gl::Uniform1ui)
    }

    fn build_sint(location: UniformLocation<GLint>, buffer: &mut [u8], value: i32) {
        Self::build_uniform(location, buffer, value, gl::Uniform1i)
    }

    fn build_float(location: UniformLocation<GLint>, buffer: &mut [u8], value: f32) {
        Self::build_uniform(location, buffer, value, gl::Uniform1f)
    }

    fn bind_texture(samplers: &SamplerSet, binding: &TextureBinding, texture: &Texture) {
        unsafe {
            // eprintln!("setting {} to texunit {}", texture.image.handle, binding.binding);
            gl::ActiveTexture(gl::TEXTURE0 + binding.binding);

            gl::BindTexture(gl::TEXTURE_2D, texture.image.handle);
            gl::BindSampler(binding.binding,
                            samplers.get(texture.wrap_mode, texture.filter, texture.mip_filter));
        }
    }

    pub fn get_format(&self) -> ShaderFormat {
        let mut fb_format = ShaderFormat::R8G8B8A8Unorm;
        if self.config.srgb_framebuffer {
            fb_format = ShaderFormat::R8G8B8A8Srgb;
        } else if self.config.float_framebuffer {
            fb_format = ShaderFormat::R16G16B16A16Sfloat;
        }
        fb_format
    }

    // todo: fix rendertargets (i.e. non-final pass is internal, final pass is user provided fbo)
    pub fn draw(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        frame_count: u32,
        frame_direction: i32,
        viewport: &Viewport,
        original: &Texture,
        source: &Texture,
        output: RenderTarget,
    ) {
        let framebuffer = output.framebuffer;

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer.handle);
            gl::UseProgram(self.program);
        }

        self.build_semantics(
            pass_index,
            parent,
            output.mvp,
            frame_count,
            frame_direction,
            framebuffer.size,
            viewport,
            original,
            source,
        );
        // shader_gl3:1514

        if self.ubo_location.vertex != gl::INVALID_INDEX
            && self.ubo_location.fragment != gl::INVALID_INDEX
        {
            if let (Some(ubo), Some(ring)) = (&self.reflection.ubo, &mut self.ubo_ring) {
                let size = ubo.size;
                let buffer = ring.current();

                unsafe {
                    gl::BindBuffer(gl::UNIFORM_BUFFER, *buffer);
                    gl::BufferSubData(
                        gl::UNIFORM_BUFFER,
                        0,
                        size as GLsizeiptr,
                        self.uniform_buffer.as_ptr().cast(),
                    );
                    gl::BindBuffer(gl::UNIFORM_BUFFER, 0);

                    if self.ubo_location.vertex != gl::INVALID_INDEX {
                        gl::BindBufferBase(gl::UNIFORM_BUFFER, self.ubo_location.vertex, *buffer);
                    }
                    if self.ubo_location.fragment != gl::INVALID_INDEX {
                        gl::BindBufferBase(gl::UNIFORM_BUFFER, self.ubo_location.fragment, *buffer);
                    }
                }
                ring.next()
            }
        }

        unsafe {
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::ClearColor(0.0f32, 0.0f32, 0.0f32, 0.0f32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            //
            gl::Viewport(
                output.x,
                output.y,
                framebuffer.size.width as GLsizei,
                framebuffer.size.height as GLsizei,
            );

            if framebuffer.format == gl::SRGB8_ALPHA8 {
                gl::Enable(gl::FRAMEBUFFER_SRGB);
            } else {
                gl::Disable(gl::FRAMEBUFFER_SRGB);
            }

            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
            gl::Disable(gl::DEPTH_TEST);

            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);

            gl::BindBuffer(gl::ARRAY_BUFFER, parent.draw_quad.vbo);

            // the provided pointers are of OpenGL provenance with respect to the buffer bound to quad_vbo,
            // and not a known provenance to the Rust abstract machine, therefore we give it invalid pointers.
            // that are inexpressible in Rust
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                (4 * std::mem::size_of::<f32>()) as GLsizei,
                std::ptr::invalid(0),
            );
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                (4 * std::mem::size_of::<f32>()) as GLsizei,
                std::ptr::invalid(2 * std::mem::size_of::<f32>()),
            );
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::DisableVertexAttribArray(0);
            gl::DisableVertexAttribArray(1);

            gl::Disable(gl::FRAMEBUFFER_SRGB);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
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
        viewport: &Viewport,
        original: &Texture,
        source: &Texture,
    ) {
        // Bind MVP
        if let Some((location, offset)) =
            self.uniform_bindings.get(&VariableSemantics::MVP.into())
        {
            let mvp_size = mvp.len() * std::mem::size_of::<f32>();
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
            };
            FilterPass::build_mat4(location.location(), &mut buffer[offset..][..mvp_size], mvp)
        }

        // bind OutputSize
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&VariableSemantics::Output.into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
            };

            FilterPass::build_vec4(location.location(), &mut buffer[offset..][..16], fb_size)
        }

        // bind FinalViewportSize
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&VariableSemantics::FinalViewport.into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
            };
            FilterPass::build_vec4(
                location.location(),
                &mut buffer[offset..][..16],
                viewport.output.size,
            )
        }

        // bind FrameCount
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&VariableSemantics::FrameCount.into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
            };
            FilterPass::build_uint(location.location(), &mut buffer[offset..][..4], frame_count)
        }

        // bind FrameDirection
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&VariableSemantics::FrameDirection.into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
            };
            FilterPass::build_sint(
                location.location(),
                &mut buffer[offset..][..4],
                frame_direction,
            )
        }

        // bind Original sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Original.semantics(0))
        {
            FilterPass::bind_texture(&parent.samplers, binding, original);
        }

        // bind OriginalSize
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&TextureSemantics::Original.semantics(0).into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
            };
            FilterPass::build_vec4(
                location.location(),
                &mut buffer[offset..][..16],
                original.image.size,
            );
        }

        // bind Source sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Source.semantics(0))
        {
            // eprintln!("setting source binding to {}", binding.binding);
            FilterPass::bind_texture(&parent.samplers, binding, source);
        }

        // bind SourceSize
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&TextureSemantics::Source.semantics(0).into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
            };
            FilterPass::build_vec4(
                location.location(),
                &mut buffer[offset..][..16],
                source.image.size,
            );
        }

        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::OriginalHistory.semantics(0))
        {
            FilterPass::bind_texture(&parent.samplers, binding, original);
        }
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&TextureSemantics::OriginalHistory.semantics(0).into())
        {
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
            };
            FilterPass::build_vec4(
                location.location(),
                &mut buffer[offset..][..16],
                original.image.size,
            );
        }

        for (index, output) in parent.history_textures.iter().enumerate() {
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::OriginalHistory.semantics(index + 1))
            {
                FilterPass::bind_texture(&parent.samplers, binding, output);
            }

            if let Some((location, offset)) = self.uniform_bindings.get(
                &TextureSemantics::OriginalHistory
                    .semantics(index + 1)
                    .into(),
            ) {
                let (buffer, offset) = match offset {
                    MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                    MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
                };
                FilterPass::build_vec4(
                    location.location(),
                    &mut buffer[offset..][..16],
                    output.image.size,
                );
            }
        }

        // PassOutput
        for (index, output) in parent.output_textures.iter().enumerate() {
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::PassOutput.semantics(index))
            {
                FilterPass::bind_texture(&parent.samplers, binding, output);
            }

            if let Some((location, offset)) = self
                .uniform_bindings
                .get(&TextureSemantics::PassOutput.semantics(index).into())
            {
                let (buffer, offset) = match offset {
                    MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                    MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
                };
                FilterPass::build_vec4(
                    location.location(),
                    &mut buffer[offset..][..16],
                    output.image.size,
                );
            }
        }

        // PassFeedback
        for (index, feedback) in parent.feedback_textures.iter().enumerate() {
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::PassFeedback.semantics(index))
            {
                if feedback.image.handle == 0 {
                    eprintln!("[WARNING] trying to bind PassFeedback: {index} which has texture 0 to slot {} in pass {pass_index}", binding.binding)
                }
                FilterPass::bind_texture(&parent.samplers, binding, feedback);
            }

            if let Some((location, offset)) = self
                .uniform_bindings
                .get(&TextureSemantics::PassFeedback.semantics(index).into())
            {
                let (buffer, offset) = match offset {
                    MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                    MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
                };
                FilterPass::build_vec4(
                    location.location(),
                    &mut buffer[offset..][..16],
                    feedback.image.size,
                );
            }
        }

        // bind float parameters
        for (id, (location, offset)) in
            self.uniform_bindings
                .iter()
                .filter_map(|(binding, value)| match binding {
                    UniformBinding::Parameter(id) => Some((id, value)),
                    _ => None,
                })
        {
            let id = id.as_str();
            let (buffer, offset) = match offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
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

            FilterPass::build_float(location.location(), &mut buffer[offset..][..4], value)
        }

        // bind luts
        for (index, lut) in &parent.luts {
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::User.semantics(*index))
            {
                FilterPass::bind_texture(&parent.samplers, binding, lut);
            }

            if let Some((location, offset)) = self
                .uniform_bindings
                .get(&TextureSemantics::User.semantics(*index).into())
            {
                let (buffer, offset) = match offset {
                    MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, *offset),
                    MemberOffset::PushConstant(offset) => (&mut self.push_buffer, *offset),
                };
                FilterPass::build_vec4(
                    location.location(),
                    &mut buffer[offset..][..16],
                    lut.image.size,
                );
            }
        }
    }
}
