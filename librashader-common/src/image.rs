use std::path::Path;

pub struct Image {
    pub bytes: Vec<u8>,
    pub size: Size<u32>,
    pub pitch: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UVDirection {
    TopLeft,
    BottomLeft,
}
impl Image {
    pub fn load(path: impl AsRef<Path>, direction: UVDirection) -> Result<Self, ImageError> {
        let mut image = image::open(path.as_ref())?;

        if direction == BottomLeft {
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

use crate::Size;
pub use image::ImageError;
use crate::image::UVDirection::BottomLeft;
