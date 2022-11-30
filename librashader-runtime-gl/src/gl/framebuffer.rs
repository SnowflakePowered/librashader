use crate::error::Result;
use crate::framebuffer::GLImage;
use crate::gl::FramebufferInterface;
use crate::texture::Texture;
use crate::Viewport;
use gl::types::{GLenum, GLuint};
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::Scale2D;

#[derive(Debug)]
pub struct Framebuffer {
    pub image: GLuint,
    pub handle: GLuint,
    pub size: Size<u32>,
    pub format: GLenum,
    pub max_levels: u32,
    pub mip_levels: u32,
    pub is_raw: bool,
}

impl Framebuffer {
    pub fn new<T: FramebufferInterface>(max_levels: u32) -> Self {
        T::new(max_levels)
    }

    pub fn new_from_raw<T: FramebufferInterface>(
        texture: GLuint,
        handle: GLuint,
        format: GLenum,
        size: Size<u32>,
        mip_levels: u32,
    ) -> Self {
        T::new_from_raw(texture, handle, format, size, mip_levels)
    }

    pub fn clear<T: FramebufferInterface, const REBIND: bool>(&self) {
        T::clear::<REBIND>(&self)
    }

    pub fn scale<T: FramebufferInterface>(
        &mut self,
        scaling: Scale2D,
        format: ImageFormat,
        viewport: &Viewport,
        original: &Texture,
        source: &Texture,
    ) -> Result<Size<u32>> {
        T::scale(self, scaling, format, viewport, original, source)
    }

    pub fn copy_from<T: FramebufferInterface>(&mut self, image: &GLImage) -> Result<()> {
        T::copy_from(self, image)
    }

    pub fn as_texture(&self, filter: FilterMode, wrap_mode: WrapMode) -> Texture {
        Texture {
            image: GLImage {
                handle: self.image,
                format: self.format,
                size: self.size,
                padded_size: Default::default(),
            },
            filter,
            mip_filter: filter,
            wrap_mode,
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            if self.handle != 0 {
                gl::DeleteFramebuffers(1, &self.handle);
            }
            if self.image != 0 {
                gl::DeleteTextures(1, &self.image);
            }
        }
    }
}
