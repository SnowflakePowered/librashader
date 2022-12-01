use librashader_common::Size;
pub use image::ImageError;

use std::path::Path;

pub struct Image {
    pub bytes: Vec<u8>,
    pub size: Size<u32>,
    pub pitch: usize,
}

/// The direction of UV coordinates to load the image for.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UVDirection {
    /// Origin is at the top left (Direct3D, Vulkan)
    TopLeft,
    /// Origin is at the bottom left (OpenGL)
    BottomLeft,
}

impl Image {
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

        Ok(Image {
            bytes: image.into_raw(),
            pitch,
            size: Size { height, width },
        })
    }
}
