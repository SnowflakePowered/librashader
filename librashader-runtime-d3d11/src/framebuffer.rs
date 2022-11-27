use windows::Win32::Graphics::Direct3D11::{D3D11_VIEWPORT, ID3D11RenderTargetView, ID3D11ShaderResourceView, ID3D11Texture2D};
use librashader_common::Size;
#[derive(Debug, Clone)]
pub struct OwnedFramebuffer {
    pub srv: ID3D11ShaderResourceView,
    pub rtv: ID3D11RenderTargetView,
    pub texture: ID3D11Texture2D,
}

#[derive(Debug, Clone)]

pub struct OutputFramebuffer {
    pub rtv: ID3D11RenderTargetView,
    pub size: Size<u32>,
    pub viewport: D3D11_VIEWPORT
}