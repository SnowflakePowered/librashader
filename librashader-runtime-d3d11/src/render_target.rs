use crate::framebuffer::OutputFramebuffer;
use crate::D3D11OutputView;
use librashader_common::Viewport;
use windows::Win32::Graphics::Direct3D11::D3D11_VIEWPORT;
use librashader_runtime::quad::DEFAULT_MVP;

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

impl<'a> From<&Viewport<'a, D3D11OutputView>> for RenderTarget<'a> {
    fn from(value: &Viewport<'a, D3D11OutputView>) -> Self {
        RenderTarget::new(
            OutputFramebuffer {
                rtv: value.output.handle.clone(),
                size: value.output.size,
                viewport: D3D11_VIEWPORT {
                    TopLeftX: value.x,
                    TopLeftY: value.y,
                    Width: value.output.size.width as f32,
                    Height: value.output.size.height as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                },
            },
            value.mvp,
        )
    }
}
