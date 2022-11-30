use crate::util;
use crate::texture::Texture;
use gl::types::{GLenum, GLint, GLsizei, GLuint};
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::{Scale2D, ScaleType, Scaling};
use crate::error::FilterChainError;
use crate::error::Result;
use crate::gl::Framebuffer;

#[derive(Debug, Copy, Clone)]
pub struct Viewport<'a, T: Framebuffer + ?Sized> {
    pub x: i32,
    pub y: i32,
    pub output: &'a T,
    pub mvp: Option<&'a [f32; 16]>,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct GLImage {
    pub handle: GLuint,
    pub format: GLenum,
    pub size: Size<u32>,
    pub padded_size: Size<u32>,
}
