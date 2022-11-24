use std::path::Path;

pub struct Image {
    pub bytes: Vec<u8>,
    pub size: Size<u32>
}

impl Image {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let image = image::open(path.as_ref())?.flipv().to_rgba8();

        let height = image.height();
        let width = image.width();

        Ok(Image {
            bytes: image.to_vec(),
            size: Size {
                height,
                width,
            }
        })
    }
}

pub use image::ImageError as ImageError;
use crate::Size;