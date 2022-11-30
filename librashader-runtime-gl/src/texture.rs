use crate::framebuffer::GLImage;
use librashader_common::{FilterMode, WrapMode};

#[derive(Default, Debug, Copy, Clone)]
pub struct Texture {
    pub image: GLImage,
    pub filter: FilterMode,
    pub mip_filter: FilterMode,
    pub wrap_mode: WrapMode,
}

impl Texture {
    pub fn is_bound(&self) -> bool {
        self.image.handle != 0
    }
}
