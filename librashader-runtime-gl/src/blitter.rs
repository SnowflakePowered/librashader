use gl::types::{GLint, GLsizei, GLuint};
use librashader_common::{FilterMode, WrapMode};
use librashader_runtime::quad::{IDENTITY_MVP, IDENTITY_MVP_FLIPY, QuadType};
use crate::{error, GLFramebuffer, GLImage, util};
use crate::error::FilterChainError;
use crate::gl::{DrawQuad, FramebufferInterface, GLInterface};
use crate::samplers::SamplerSet;

pub struct Blitter {
    program: GLuint,
    tex_loc: GLint,
    mvp_loc: GLint,
}

const BLIT_VERT: &str = include_str!("../shader/blit.vert.glsl");

const BLIT_FRAG: &str = include_str!("../shader/blit.frag.glsl");
impl Blitter {
    pub fn new() -> error::Result<Self> {
        unsafe  {
            let vertex = util::gl_compile_shader(gl::VERTEX_SHADER, BLIT_VERT)?;
            let fragment = util::gl_compile_shader(gl::FRAGMENT_SHADER, BLIT_FRAG)?;

            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex);
            gl::AttachShader(program, fragment);

            gl::BindAttribLocation(program, 0, b"Position\0".as_ptr().cast());
            gl::BindAttribLocation(program, 1, b"TexCoord\0".as_ptr().cast());

            gl::LinkProgram(program);
            gl::DeleteShader(vertex);
            gl::DeleteShader(fragment);

            let mut status = 0;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
            if status != 1 {
                return Err(FilterChainError::GLLinkError);
            }

            let tex_loc = gl::GetUniformLocation(program, b"Texture\0".as_ptr().cast());
            let mvp_loc = gl::GetUniformLocation(program, b"MVP\0".as_ptr().cast());

            Ok(Self {
                program,
                tex_loc,
                mvp_loc
            })
        }
    }

    pub fn blit<T: GLInterface>(&self, sampler_set: &SamplerSet,
                                drawquad: &<T as GLInterface>::DrawQuad, dest: &mut GLFramebuffer,
                                source: &GLImage, flipy: bool) -> error::Result<()>{
        dest.right_size::<T::FramebufferInterface>(source)?;
        self.blit_unchecked( sampler_set, drawquad, dest, source.handle, flipy);
        Ok(())
    }

    pub fn blit_unchecked<T: DrawQuad>(&self, sampler_set: &SamplerSet, drawquad: &T, dest: &GLFramebuffer, source: GLuint, flipy: bool) {
        unsafe {
            gl::UseProgram(self.program);
            gl::Uniform1i(self.tex_loc, 0);

            gl::ActiveTexture(gl::TEXTURE0);

            gl::BindFramebuffer(gl::FRAMEBUFFER, dest.fbo);
            gl::BindTexture(gl::TEXTURE_2D, source);
            gl::BindSampler(0, sampler_set.get(WrapMode::ClampToEdge, FilterMode::Nearest, FilterMode::Nearest));

            assert!(source != 0);

            gl::Viewport(0, 0, dest.size.width as GLsizei, dest.size.height as GLsizei);
            // gl::Clear(gl::COLOR_BUFFER_BIT);


            let mvp = if flipy { IDENTITY_MVP_FLIPY } else { IDENTITY_MVP };

            gl::UniformMatrix4fv(self.mvp_loc, 1, gl::FALSE, mvp.as_ptr().cast());

            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
            gl::Disable(gl::DEPTH_TEST);


            drawquad.bind_vertices(QuadType::Offscreen);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            drawquad.unbind_vertices();
            gl::UseProgram(0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }
}