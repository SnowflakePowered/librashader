use crate::texture::D3D12OutputView;

pub(crate) struct RenderTarget<'a> {
    pub x: f32,
    pub y: f32,
    pub mvp: &'a [f32; 16],
    pub output: D3D12OutputView,
}
