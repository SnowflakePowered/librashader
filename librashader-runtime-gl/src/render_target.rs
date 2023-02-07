use crate::gl::Framebuffer;

#[rustfmt::skip]
pub static GL_MVP_DEFAULT: &[f32; 16] = &[
    2f32, 0.0, 0.0, 0.0,
    0.0, 2.0, 0.0, 0.0,
    0.0, 0.0, 2.0, 0.0,
    -1.0, -1.0, 0.0, 1.0,
];

#[derive(Debug, Copy, Clone)]
pub(crate) struct RenderTarget<'a> {
    pub mvp: &'a [f32; 16],
    pub framebuffer: &'a Framebuffer,
    pub x: i32,
    pub y: i32,
}

impl<'a> RenderTarget<'a> {
    pub fn new(backbuffer: &'a Framebuffer, mvp: &'a [f32; 16], x: i32, y: i32) -> Self {
        RenderTarget {
            framebuffer: backbuffer,
            x,
            mvp,
            y,
        }
    }
}
