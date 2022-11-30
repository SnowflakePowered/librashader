use std::path::Path;

pub struct Image {
    pub bytes: Vec<u8>,
    pub size: Size<u32>,
    pub pitch: usize,
}

impl Image {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let image = image::open(path.as_ref())?.flipv().to_rgba8();

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
