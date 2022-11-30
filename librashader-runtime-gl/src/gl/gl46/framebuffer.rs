use gl::types::{GLenum, GLint, GLsizei, GLuint};
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::Scale2D;
use crate::framebuffer::{GLImage, Viewport};
use crate::error::{FilterChainError, Result};
use crate::gl::Framebuffer;
use crate::texture::Texture;

#[derive(Debug)]
pub struct Gl46Framebuffer {
    image: GLuint,
    handle: GLuint,
    size: Size<u32>,
    format: GLenum,
    max_levels: u32,
    levels: u32,
    is_raw: bool,
}


impl Framebuffer for Gl46Framebuffer {
    fn handle(&self) -> GLuint {
        self.handle
    }

    fn size(&self) -> Size<u32> {
        self.size
    }

    fn image(&self) -> GLuint {
        self.image
    }

    fn format(&self) -> GLenum {
        self.format
    }

    fn new(max_levels: u32) -> Gl46Framebuffer {
        let mut framebuffer = 0;
        unsafe {
            gl::CreateFramebuffers(1, &mut framebuffer);
        }

        Gl46Framebuffer {
            image: 0,
            size: Size {
                width: 1,
                height: 1,
            },
            format: 0,
            max_levels,
            levels: 0,
            handle: framebuffer,
            is_raw: false,
        }
    }
    fn new_from_raw(
        texture: GLuint,
        handle: GLuint,
        format: GLenum,
        size: Size<u32>,
        miplevels: u32,
    ) -> Gl46Framebuffer {
        Gl46Framebuffer {
            image: texture,
            size,
            format,
            max_levels: miplevels,
            levels: miplevels,
            handle,
            is_raw: true,
        }
    }
    fn as_texture(&self, filter: FilterMode, wrap_mode: WrapMode) -> Texture {
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
    fn scale(
        &mut self,
        scaling: Scale2D,
        format: ImageFormat,
        viewport: &Viewport<Self>,
        _original: &Texture,
        source: &Texture,
    ) -> Result<Size<u32>> {
        if self.is_raw {
            return Ok(self.size);
        }

        let size = librashader_runtime::scaling::scale(scaling, source.image.size, viewport.output.size);

        if self.size != size {
            self.size = size;

            self.init(
                size,
                if format == ImageFormat::Unknown {
                    ImageFormat::R8G8B8A8Unorm
                } else {
                    format
                },
            )?;
        }
        Ok(size)
    }
    fn clear<const REBIND: bool>(&self) {
        unsafe {
            gl::ClearNamedFramebufferfv(self.handle,
                                        gl::COLOR, 0,
                                        [0.0f32, 0.0, 0.0, 0.0].as_ptr().cast());
        }
    }
    fn copy_from(&mut self, image: &GLImage) -> Result<()> {
        // todo: may want to use a shader and draw a quad to be faster.
        if image.size != self.size || image.format != self.format {
            self.init(image.size, image.format)?;
        }

        unsafe {
            // gl::NamedFramebufferDrawBuffer(self.handle, gl::COLOR_ATTACHMENT1);
            gl::NamedFramebufferReadBuffer(image.handle, gl::COLOR_ATTACHMENT0);
            gl::NamedFramebufferDrawBuffer(self.handle, gl::COLOR_ATTACHMENT1);

            gl::BlitNamedFramebuffer(image.handle, self.handle,
                                     0, 0, image.size.width as GLint, image.size.height as GLint,
                                     0, 0, self.size.width as GLint, self.size.height as GLint,
                                     gl::COLOR_BUFFER_BIT, gl::NEAREST);

        }

        Ok(())
    }
    fn init(&mut self, mut size: Size<u32>, format: impl Into<GLenum>) -> Result<()> {
        if self.is_raw {
            return Ok(());
        }
        self.format = format.into();
        self.size = size;

        unsafe {
            // reset the framebuffer image
            if self.image != 0 {
                gl::NamedFramebufferTexture(
                    self.handle,
                    gl::COLOR_ATTACHMENT0,
                    0,
                    0,
                );
                gl::DeleteTextures(1, &self.image);
            }

            gl::CreateTextures(gl::TEXTURE_2D,1, &mut self.image);

            if size.width == 0 {
                size.width = 1;
            }
            if size.height == 0 {
                size.height = 1;
            }

            self.levels = librashader_runtime::scaling::calc_miplevel(size);
            if self.levels > self.max_levels {
                self.levels = self.max_levels;
            }
            if self.levels == 0 {
                self.levels = 1;
            }

            gl::TextureStorage2D(
                self.image,
                self.levels as GLsizei,
                self.format,
                size.width as GLsizei,
                size.height as GLsizei,
            );

            gl::NamedFramebufferTexture(
                self.handle,
                gl::COLOR_ATTACHMENT0,
                self.image,
                0,
            );

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                match status {
                    gl::FRAMEBUFFER_UNSUPPORTED => {
                        eprintln!("unsupported fbo");

                        gl::NamedFramebufferTexture(
                            self.handle,
                            gl::COLOR_ATTACHMENT0,
                            0,
                            0,
                        );
                        gl::DeleteTextures(1, &self.image);
                        gl::CreateTextures(gl::TEXTURE_2D, 1, &mut self.image);

                        self.levels = librashader_runtime::scaling::calc_miplevel(size);
                        if self.levels > self.max_levels {
                            self.levels = self.max_levels;
                        }
                        if self.levels == 0 {
                            self.levels = 1;
                        }

                        gl::TextureStorage2D(
                            self.image,
                            self.levels as GLsizei,
                            ImageFormat::R8G8B8A8Unorm.into(),
                            size.width as GLsizei,
                            size.height as GLsizei,
                        );
                        gl::NamedFramebufferTexture(
                            self.handle,
                            gl::COLOR_ATTACHMENT0,
                            self.image,
                            0,
                        );
                        // self.init =
                        //     gl::CheckFramebufferStatus(gl::FRAMEBUFFER) == gl::FRAMEBUFFER_COMPLETE;
                    }
                    _ => return Err(FilterChainError::FramebufferInit(status))
                }
            }
        }
        Ok(())
    }
}

impl Drop for Gl46Framebuffer {
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