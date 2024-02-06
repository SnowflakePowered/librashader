use crate::handle::Handle;
use crate::texture::OwnedImage;
use librashader_common::Size;
use wgpu::TextureViewDescriptor;

/// A wgpu `TextureView` with size and texture information to output.
pub struct OutputView<'a> {
    pub(crate) size: Size<u32>,
    pub(crate) view: Handle<'a, wgpu::TextureView>,
    pub(crate) format: wgpu::TextureFormat,
}

impl<'a> OutputView<'a> {
    pub fn new_from_raw(
        view: &'a wgpu::TextureView,
        size: Size<u32>,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            size,
            view: Handle::Borrowed(&view),
            format,
        }
    }
}

impl<'a> From<&'a OwnedImage> for OutputView<'a> {
    fn from(image: &'a OwnedImage) -> Self {
        Self {
            size: image.size,
            view: Handle::Borrowed(&image.view),
            format: image.image.format(),
        }
    }
}

impl From<&wgpu::Texture> for OutputView<'static> {
    fn from(image: &wgpu::Texture) -> Self {
        Self {
            size: image.size().into(),
            view: Handle::Owned(image.create_view(&TextureViewDescriptor::default())),
            format: image.format(),
        }
    }
}
