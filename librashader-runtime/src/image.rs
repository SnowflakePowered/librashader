pub use image::ImageError;
use librashader_common::Size;
use std::marker::PhantomData;

use std::path::Path;

pub struct Image<P: PixelFormat = RGBA8> {
    pub bytes: Vec<u8>,
    pub size: Size<u32>,
    pub pitch: usize,
    _pd: PhantomData<P>,
}

pub struct RGBA8;
pub struct BGRA8;

pub trait PixelFormat {
    #[doc(hidden)]
    fn convert(pixels: &mut Vec<u8>);
}

impl PixelFormat for RGBA8 {
    fn convert(_pixels: &mut Vec<u8>) {}
}

impl PixelFormat for BGRA8 {
    fn convert(pixels: &mut Vec<u8>) {
        for [r, _g, b, _a] in pixels.array_chunks_mut::<4>() {
            std::mem::swap(b, r)
        }
    }
}

/// The direction of UV coordinates to load the image for.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UVDirection {
    /// Origin is at the top left (Direct3D, Vulkan)
    TopLeft,
    /// Origin is at the bottom left (OpenGL)
    BottomLeft,
}

impl<P: PixelFormat> Image<P> {
    /// Load the image from the path as RGBA8.
    pub fn load(path: impl AsRef<Path>, direction: UVDirection) -> Result<Self, ImageError> {
        let mut image = image::open(path.as_ref())?;

        if direction == UVDirection::BottomLeft {
            image = image.flipv();
        }

        let image = image.to_rgba8();

        let height = image.height();
        let width = image.width();
        let pitch = image
            .sample_layout()
            .height_stride
            .max(image.sample_layout().width_stride);

        let mut bytes = image.into_raw();
        P::convert(&mut bytes);
        Ok(Image {
            bytes,
            pitch,
            size: Size { height, width },
            _pd: Default::default(),
        })
    }
}
