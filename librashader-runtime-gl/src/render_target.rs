use crate::framebuffer::Framebuffer;
use crate::util::Viewport;

#[rustfmt::skip]
static DEFAULT_MVP: &[f32] = &[
    2f32, 0.0, 0.0, 0.0,
    0.0, 2.0, 0.0, 0.0,
    0.0, 0.0, 2.0, 0.0,
    -1.0, -1.0, 0.0, 1.0,
];

#[derive(Debug, Copy, Clone)]
pub struct RenderTarget<'a> {
    pub mvp: &'a [f32],
    pub framebuffer: &'a Framebuffer,
}

impl<'a> RenderTarget<'a> {
    pub fn new(backbuffer: &'a Framebuffer, mvp: Option<&'a [f32]>) -> Self {
        if let Some(mvp) = mvp {
            RenderTarget {
                framebuffer: backbuffer,
                mvp,
            }
        } else {
            RenderTarget {
                framebuffer: backbuffer,
                mvp: DEFAULT_MVP,
            }
        }
    }
}

impl<'a> From<&Viewport<'a>> for RenderTarget<'a> {
    fn from(value: &Viewport<'a>) -> Self {
        RenderTarget::new(value.output, value.mvp)
    }
}
