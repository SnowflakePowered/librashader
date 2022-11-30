use crate::framebuffer::Viewport;
use crate::gl::Framebuffer;

#[rustfmt::skip]
static DEFAULT_MVP: &[f32; 16] = &[
    2f32, 0.0, 0.0, 0.0,
    0.0, 2.0, 0.0, 0.0,
    0.0, 0.0, 2.0, 0.0,
    -1.0, -1.0, 0.0, 1.0,
];

#[derive(Debug, Copy, Clone)]
pub(crate) struct RenderTarget<'a, T: Framebuffer> {
    pub mvp: &'a [f32; 16],
    pub framebuffer: &'a T,
    pub x: i32,
    pub y: i32
}

impl<'a, T: Framebuffer> RenderTarget<'a, T> {
    pub fn new(backbuffer: &'a T, mvp: Option<&'a [f32; 16]>, x: i32, y: i32) -> Self {
        if let Some(mvp) = mvp {
            RenderTarget {
                framebuffer: backbuffer,
                x,
                mvp,
                y,
            }
        } else {
            RenderTarget {
                framebuffer: backbuffer,
                x,
                mvp: DEFAULT_MVP,
                y,
            }
        }
    }
}

impl<'a, T: Framebuffer> From<&Viewport<'a, T>> for RenderTarget<'a, T> {
    fn from(value: &Viewport<'a, T>) -> Self {
        RenderTarget::new(value.output, value.mvp, value.x, value.y)
    }
}
