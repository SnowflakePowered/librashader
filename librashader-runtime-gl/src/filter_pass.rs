use gl::types::{GLint, GLsizei, GLuint};
use librashader_reflect::back::cross::CrossGlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::ShaderReflection;

use librashader_common::{ImageFormat, Size, Viewport};
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::reflect::semantics::{MemberOffset, TextureBinding, TextureSemantics, UniformBinding, UniqueSemantics};
use rustc_hash::FxHashMap;
use librashader_runtime::binding::{BindSemantics, ContextOffset, TextureInput};

use crate::binding::{GlUniformBinder, GlUniformStorage, UniformLocation, VariableLocation};
use crate::filter_chain::FilterCommon;
use crate::Framebuffer;
use crate::gl::{BindTexture, GLInterface, UboRing};
use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;

use crate::texture::InputTexture;

pub struct UniformOffset {
    pub location: VariableLocation,
    pub offset: MemberOffset
}

impl UniformOffset {
    pub fn new(location: VariableLocation, offset: MemberOffset) -> Self {
        Self {
            location,
            offset
        }
    }
}

pub struct FilterPass<T: GLInterface> {
    pub reflection: ShaderReflection,
    pub compiled: ShaderCompilerOutput<String, CrossGlslContext>,
    pub program: GLuint,
    pub ubo_location: UniformLocation<GLuint>,
    pub ubo_ring: Option<T::UboRing>,
    pub(crate) uniform_storage: GlUniformStorage,
    pub uniform_bindings: FxHashMap<UniformBinding, UniformOffset>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
}

impl TextureInput for InputTexture {
    fn size(&self) -> Size<u32> {
        self.image.size
    }
}

impl ContextOffset<GlUniformBinder, UniformLocation<GLint>> for UniformOffset {
    fn offset(&self) -> MemberOffset {
        self.offset
    }

    fn context(&self) -> UniformLocation<GLint> {
        self.location.location()
    }
}

impl<T: GLInterface> BindSemantics<GlUniformBinder, UniformLocation<GLint>> for FilterPass<T> {
    type InputTexture = InputTexture;
    type SamplerSet = SamplerSet;
    type DescriptorSet<'a> = ();
    type DeviceContext = ();
    type UniformOffset = UniformOffset;

    fn bind_texture<'a>(
        _descriptors: &mut Self::DescriptorSet<'a>, samplers: &Self::SamplerSet,
        binding: &TextureBinding, texture: &Self::InputTexture,
        _device: &Self::DeviceContext) {
        T::BindTexture::bind_texture(&samplers, binding, texture);
    }
}

impl<T: GLInterface> FilterPass<T> {
    pub(crate) fn draw(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        frame_count: u32,
        frame_direction: i32,
        viewport: &Viewport<&Framebuffer>,
        original: &InputTexture,
        source: &InputTexture,
        output: RenderTarget,
    ) {
        let framebuffer = output.framebuffer;

        if self.config.mipmap_input && !parent.disable_mipmaps {
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
    fn build_semantics(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        mvp: &[f32; 16],
        frame_count: u32,
        frame_direction: i32,
        fb_size: Size<u32>,
        viewport: &Viewport<&Framebuffer>,
        original: &InputTexture,
        source: &InputTexture,
    ) {
        Self::bind_semantics(
            &(),
            &parent.samplers,
            &mut self.uniform_storage,
            &mut (),
            mvp,
            frame_count,
            frame_direction,
            fb_size,
            viewport.output.size,
            original,
            source,
            &self.uniform_bindings,
            &self.reflection.meta.texture_meta,
            parent.output_textures[0..pass_index].iter()
                .map(|o| o.bound()),
            parent.feedback_textures.iter()
                .map(|o| o.bound()),
            parent.history_textures.iter()
                .map(|o| o.bound()),
            parent.luts.iter()
                .map(|(u, i)| (*u, i)),
            &self.source.parameters,
            &parent.config.parameters
        );
    }
}
