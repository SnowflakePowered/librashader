use librashader_common::{FilterMode, WrapMode};
use crate::GlImage;

#[derive(Default, Debug, Copy, Clone)]
pub struct Texture {
    pub image: GlImage,
    pub filter: FilterMode,
    pub mip_filter: FilterMode,
    pub wrap_mode: WrapMode,
}

impl Texture {
    pub fn is_bound(&self) -> bool {
        return self.image.handle != 0
    }
}