use crate::texture::OwnedImage;
use librashader_common::Size;

pub struct OutputView<'a> {
    pub size: Size<u32>,
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
}

impl<'a> OutputView<'a> {
    pub fn new(image: &'a OwnedImage) -> Self {
        Self {
            size: image.size,
            view: &image.view,
            format: image.image.format(),
        }
    }
}
