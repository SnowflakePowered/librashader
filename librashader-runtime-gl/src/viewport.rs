use crate::gl::Framebuffer;

#[derive(Debug, Copy, Clone)]
pub struct Viewport<'a> {
    pub x: f32,
    pub y: f32,
    pub output: &'a Framebuffer,
    pub mvp: Option<&'a [f32; 16]>,
}
