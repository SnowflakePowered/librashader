use wgpu::TextureView;
use librashader_common::Size;

pub(crate) struct OutputImage {
    pub size: Size<u32>,
    pub view: TextureView
}