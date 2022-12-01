use librashader_common::Size;
use windows::Win32::Graphics::Direct3D11::ID3D11RenderTargetView;

#[derive(Debug, Clone)]
pub struct Viewport<'a> {
    pub x: f32,
    pub y: f32,
    pub size: Size<u32>,
    pub output: ID3D11RenderTargetView,
    pub mvp: Option<&'a [f32; 16]>,
}
