use librashader_common::Viewport;
use crate::gl::Framebuffer;

#[rustfmt::skip]
static DEFAULT_MVP: &[f32; 16] = &[
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
    pub fn new(backbuffer: &'a Framebuffer, mvp: Option<&'a [f32; 16]>, x: i32, y: i32) -> Self {
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

impl<'a> From<&Viewport<'a, &'a Framebuffer>> for RenderTarget<'a> {
    fn from(value: &Viewport<'a, &'a Framebuffer>) -> Self {
        RenderTarget::new(value.output, value.mvp, value.x as i32, value.y as i32)
    }
}
