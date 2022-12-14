use crate::framebuffer::OutputFramebuffer;
use crate::viewport::Viewport;
use windows::Win32::Graphics::Direct3D11::D3D11_VIEWPORT;

#[rustfmt::skip]
static DEFAULT_MVP: &[f32; 16] = &[
    2f32, 0.0, 0.0, 0.0,
    0.0, 2.0, 0.0, 0.0,
    0.0, 0.0, 2.0, 0.0,
    -1.0, -1.0, 0.0, 1.0,
];

#[derive(Debug, Clone)]
pub(crate) struct RenderTarget<'a> {
    pub mvp: &'a [f32; 16],
    pub output: OutputFramebuffer,
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

impl<'a> From<&Viewport<'a>> for RenderTarget<'a> {
    fn from(value: &Viewport<'a>) -> Self {
        RenderTarget::new(
            OutputFramebuffer {
                rtv: value.output.clone(),
                size: value.size,
                viewport: D3D11_VIEWPORT {
                    TopLeftX: value.x,
                    TopLeftY: value.y,
                    Width: value.size.width as f32,
                    Height: value.size.height as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                },
            },
            value.mvp,
        )
    }
}
