use gl::types::{GLenum, GLuint};
use librashader_common::Size;

use crate::gl::{Framebuffer, FramebufferInterface};

#[derive(Debug, Copy, Clone)]
pub struct Viewport<'a> {
    pub x: i32,
    pub y: i32,
    pub output: &'a Framebuffer,
    pub mvp: Option<&'a [f32; 16]>,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct GLImage {
    pub handle: GLuint,
    pub format: GLenum,
    pub size: Size<u32>,
    pub padded_size: Size<u32>,
}
