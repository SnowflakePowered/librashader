use librashader_common::Size;
use crate::texture::OwnedImage;

pub struct OutputImage<'a> {
    pub size: Size<u32>,
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
}

impl<'a> OutputImage<'a> {
    pub fn new(image: &'a OwnedImage) -> Self {
        Self {
            size: image.size,
            view: &image.view,
            format: image.image.format()
        }
    }
}