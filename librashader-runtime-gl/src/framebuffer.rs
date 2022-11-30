use crate::util;
use crate::texture::Texture;
use gl::types::{GLenum, GLint, GLsizei, GLuint};
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::{Scale2D, ScaleType, Scaling};
use crate::error::FilterChainError;
use crate::error::Result;
use crate::gl::Framebuffer;
use crate::gl::gl3::Gl3Framebuffer;

#[derive(Debug, Copy, Clone)]
pub struct Viewport<'a> {
    pub x: i32,
    pub y: i32,
    pub output: &'a Gl3Framebuffer,
    pub mvp: Option<&'a [f32; 16]>,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct GlImage {
    pub handle: GLuint,
    pub format: GLenum,
    pub size: Size<u32>,
    pub padded_size: Size<u32>,
}
