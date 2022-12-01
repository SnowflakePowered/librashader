use gl::types::{GLsizei, GLuint};
use librashader_reflect::back::cross::GlslangGlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::ShaderReflection;

use librashader_common::{ImageFormat, Size};
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::reflect::semantics::{
    MemberOffset, TextureSemantics, UniformBinding, VariableSemantics,
};
use rustc_hash::FxHashMap;

use crate::binding::{BufferStorage, UniformLocation, VariableLocation};
use crate::filter_chain::FilterCommon;
use crate::gl::{BindTexture, GLInterface, UboRing};
use crate::render_target::RenderTarget;
use crate::viewport::Viewport;

use crate::texture::Texture;

pub struct FilterPass<T: GLInterface> {
    pub reflection: ShaderReflection,
    pub compiled: ShaderCompilerOutput<String, GlslangGlslContext>,
    pub program: GLuint,
    pub ubo_location: UniformLocation<GLuint>,
    pub ubo_ring: Option<T::UboRing>,
    pub(crate) uniform_storage: BufferStorage,
    pub uniform_bindings: FxHashMap<UniformBinding, (VariableLocation, MemberOffset)>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
}

impl<T: GLInterface> FilterPass<T> {
    pub(crate) fn draw(
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

        if self.config.mipmap_input {
            T::BindTexture::gen_mipmaps(source);
        }

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

        if self.ubo_location.vertex != gl::INVALID_INDEX
            && self.ubo_location.fragment != gl::INVALID_INDEX
        {
            if let (Some(ubo), Some(ring)) = (&self.reflection.ubo, &mut self.ubo_ring) {
                ring.bind_for_frame(ubo, &self.ubo_location, &self.uniform_storage)
            }
        }

        unsafe {
            framebuffer.clear::<T::FramebufferInterface, false>();

            let framebuffer_size = framebuffer.size;
            gl::Viewport(
                output.x,
                output.y,
                framebuffer_size.width as GLsizei,
                framebuffer_size.height as GLsizei,
            );

            if framebuffer.format == gl::SRGB8_ALPHA8 {
                gl::Enable(gl::FRAMEBUFFER_SRGB);
            } else {
                gl::Disable(gl::FRAMEBUFFER_SRGB);
            }

            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
            gl::Disable(gl::DEPTH_TEST);

            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::Disable(gl::FRAMEBUFFER_SRGB);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }
}

impl<T: GLInterface> FilterPass<T> {
    pub fn get_format(&self) -> ImageFormat {
        let mut fb_format = ImageFormat::R8G8B8A8Unorm;
        if self.config.srgb_framebuffer {
            fb_format = ImageFormat::R8G8B8A8Srgb;
        } else if self.config.float_framebuffer {
            fb_format = ImageFormat::R16G16B16A16Sfloat;
        }
        fb_format
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
        if let Some((location, offset)) = self.uniform_bindings.get(&VariableSemantics::MVP.into())
        {
            self.uniform_storage
                .bind_mat4(*offset, mvp, location.location());
        }

        // bind OutputSize
        if let Some((location, offset)) =
            self.uniform_bindings.get(&VariableSemantics::Output.into())
        {
            self.uniform_storage
                .bind_vec4(*offset, fb_size, location.location());
        }

        // bind FinalViewportSize
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&VariableSemantics::FinalViewport.into())
        {
            self.uniform_storage
                .bind_vec4(*offset, viewport.output.size, location.location());
        }

        // bind FrameCount
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&VariableSemantics::FrameCount.into())
        {
            self.uniform_storage
                .bind_scalar(*offset, frame_count, location.location());
        }

        // bind FrameDirection
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&VariableSemantics::FrameDirection.into())
        {
            self.uniform_storage
                .bind_scalar(*offset, frame_direction, location.location());
        }

        // bind Original sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Original.semantics(0))
        {
            T::BindTexture::bind_texture(&parent.samplers, binding, original);
        }

        // bind OriginalSize
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&TextureSemantics::Original.semantics(0).into())
        {
            self.uniform_storage
                .bind_vec4(*offset, original.image.size, location.location());
        }

        // bind Source sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Source.semantics(0))
        {
            // eprintln!("setting source binding to {}", binding.binding);
            T::BindTexture::bind_texture(&parent.samplers, binding, source);
        }

        // bind SourceSize
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&TextureSemantics::Source.semantics(0).into())
        {
            self.uniform_storage
                .bind_vec4(*offset, source.image.size, location.location());
        }

        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::OriginalHistory.semantics(0))
        {
            T::BindTexture::bind_texture(&parent.samplers, binding, original);
        }
        if let Some((location, offset)) = self
            .uniform_bindings
            .get(&TextureSemantics::OriginalHistory.semantics(0).into())
        {
            self.uniform_storage
                .bind_vec4(*offset, original.image.size, location.location());
        }

        for (index, output) in parent.history_textures.iter().enumerate() {
            if !output.is_bound() {
                continue;
            }
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::OriginalHistory.semantics(index + 1))
            {
                T::BindTexture::bind_texture(&parent.samplers, binding, output);
            }

            if let Some((location, offset)) = self.uniform_bindings.get(
                &TextureSemantics::OriginalHistory
                    .semantics(index + 1)
                    .into(),
            ) {
                self.uniform_storage
                    .bind_vec4(*offset, output.image.size, location.location());
            }
        }

        // PassOutput
        for (index, output) in parent.output_textures[0..pass_index].iter().enumerate() {
            if !output.is_bound() {
                continue;
            }
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::PassOutput.semantics(index))
            {
                T::BindTexture::bind_texture(&parent.samplers, binding, output);
            }

            if let Some((location, offset)) = self
                .uniform_bindings
                .get(&TextureSemantics::PassOutput.semantics(index).into())
            {
                self.uniform_storage
                    .bind_vec4(*offset, output.image.size, location.location());
            }
        }

        // PassFeedback
        for (index, feedback) in parent.feedback_textures.iter().enumerate() {
            if !feedback.is_bound() {
                continue;
            }
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::PassFeedback.semantics(index))
            {
                T::BindTexture::bind_texture(&parent.samplers, binding, feedback);
            }

            if let Some((location, offset)) = self
                .uniform_bindings
                .get(&TextureSemantics::PassFeedback.semantics(index).into())
            {
                self.uniform_storage
                    .bind_vec4(*offset, feedback.image.size, location.location());
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
            // presets override params
            let default = self
                .source
                .parameters
                .iter()
                .find(|&p| p.id == id)
                .map(|f| f.initial)
                .unwrap_or(0f32);

            let value = *parent.config.parameters.get(id).unwrap_or(&default);

            self.uniform_storage
                .bind_scalar(*offset, value, location.location());
        }

        // bind luts
        for (index, lut) in &parent.luts {
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::User.semantics(*index))
            {
                T::BindTexture::bind_texture(&parent.samplers, binding, lut);
            }

            if let Some((location, offset)) = self
                .uniform_bindings
                .get(&TextureSemantics::User.semantics(*index).into())
            {
                self.uniform_storage
                    .bind_vec4(*offset, lut.image.size, location.location());
            }
        }
    }
}
