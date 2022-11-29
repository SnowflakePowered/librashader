use windows::Win32::Graphics::Direct3D11::ID3D11RenderTargetView;
use librashader_common::Size;
use crate::framebuffer::{OutputFramebuffer};

#[rustfmt::skip]
static DEFAULT_MVP: &[f32; 16] = &[
    2f32, 0.0, 0.0, 0.0,
    0.0, 2.0, 0.0, 0.0,
    0.0, 0.0, 2.0, 0.0,
    -1.0, -1.0, 0.0, 1.0,
];

#[derive(Debug, Clone)]
pub struct RenderTarget<'a> {
    pub mvp: &'a [f32; 16],
    pub output: OutputFramebuffer
}

impl<'a> RenderTarget<'a> {
    pub fn new(backbuffer: OutputFramebuffer, mvp: Option<&'a [f32; 16]>) -> Self {
        if let Some(mvp) = mvp {
            RenderTarget {
                output: backbuffer,
                mvp,
            }
        } else {
            RenderTarget {
                output: backbuffer,
                mvp: DEFAULT_MVP,
            }
        }
    }
}

