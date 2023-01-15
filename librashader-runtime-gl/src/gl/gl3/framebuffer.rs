use crate::error::{FilterChainError, Result};
use crate::framebuffer::GLImage;
use crate::gl::framebuffer::Framebuffer;
use crate::gl::FramebufferInterface;
use crate::texture::InputTexture;
use gl::types::{GLenum, GLint, GLsizei};
use librashader_common::{ImageFormat, Size, Viewport};
use librashader_presets::Scale2D;
use librashader_runtime::scaling::{MipmapSize, ViewportSize};

#[derive(Debug)]
pub struct Gl3Framebuffer;

impl FramebufferInterface for Gl3Framebuffer {
    fn new(max_levels: u32) -> Framebuffer {
        let mut framebuffer = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        Framebuffer {
            image: 0,
            size: Size {
                width: 1,
                height: 1,
            },
            format: 0,
            max_levels,
            mip_levels: 0,
            handle: framebuffer,
            is_raw: false,
        }
    }

    fn scale(
        fb: &mut Framebuffer,
        scaling: Scale2D,
        format: ImageFormat,
        viewport: &Viewport<&Framebuffer>,
        _original: &InputTexture,
        source: &InputTexture,
        mipmap: bool,
    ) -> Result<Size<u32>> {
        if fb.is_raw {
            return Ok(fb.size);
        }

        let size = source
            .image
            .size
            .scale_viewport(scaling, viewport.output.size);

        if fb.size != size || (mipmap && fb.max_levels == 1) || (!mipmap && fb.max_levels != 1) {
            fb.size = size;
            if mipmap {
                fb.max_levels = u32::MAX;
            } else {
                fb.max_levels = 1
            }
            Self::init(
                fb,
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
    fn clear<const REBIND: bool>(fb: &Framebuffer) {
        unsafe {
            if REBIND {
                gl::BindFramebuffer(gl::FRAMEBUFFER, fb.handle);
            }
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            if REBIND {
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            }
        }
    }
    fn copy_from(fb: &mut Framebuffer, image: &GLImage) -> Result<()> {
        // todo: may want to use a shader and draw a quad to be faster.
        if image.size != fb.size || image.format != fb.format {
            Self::init(fb, image.size, image.format)?;
        }

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, fb.handle);

            gl::FramebufferTexture2D(
                gl::READ_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                image.handle,
                0,
            );

            gl::FramebufferTexture2D(
                gl::DRAW_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT1,
                gl::TEXTURE_2D,
                fb.image,
                0,
            );
            gl::ReadBuffer(gl::COLOR_ATTACHMENT0);
            gl::DrawBuffer(gl::COLOR_ATTACHMENT1);
            gl::BlitFramebuffer(
                0,
                0,
                fb.size.width as GLint,
                fb.size.height as GLint,
                0,
                0,
                fb.size.width as GLint,
                fb.size.height as GLint,
                gl::COLOR_BUFFER_BIT,
                gl::NEAREST,
            );

            // cleanup after ourselves.
            gl::FramebufferTexture2D(
                gl::READ_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                0,
                0,
            );

            gl::FramebufferTexture2D(
                gl::DRAW_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT1,
                gl::TEXTURE_2D,
                0,
                0,
            );

            // set this back to color_attachment 0
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                fb.image,
                0,
            );

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        Ok(())
    }
    fn init(fb: &mut Framebuffer, mut size: Size<u32>, format: impl Into<GLenum>) -> Result<()> {
        if fb.is_raw {
            return Ok(());
        }
        fb.format = format.into();
        fb.size = size;

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, fb.handle);

            // reset the framebuffer image
            if fb.image != 0 {
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    0,
                    0,
                );
                gl::DeleteTextures(1, &fb.image);
            }

            gl::GenTextures(1, &mut fb.image);
            gl::BindTexture(gl::TEXTURE_2D, fb.image);

            if size.width == 0 {
                size.width = 1;
            }
            if size.height == 0 {
                size.height = 1;
            }

            fb.mip_levels = size.calculate_miplevels();
            if fb.mip_levels > fb.max_levels {
                fb.mip_levels = fb.max_levels;
            }
            if fb.mip_levels == 0 {
                fb.mip_levels = 1;
            }

            gl::TexStorage2D(
                gl::TEXTURE_2D,
                fb.mip_levels as GLsizei,
                fb.format,
                size.width as GLsizei,
                size.height as GLsizei,
            );

            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                fb.image,
                0,
            );

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                match status {
                    gl::FRAMEBUFFER_UNSUPPORTED => {
                        eprintln!("unsupported fbo");

                        gl::FramebufferTexture2D(
                            gl::FRAMEBUFFER,
                            gl::COLOR_ATTACHMENT0,
                            gl::TEXTURE_2D,
                            0,
                            0,
                        );
                        gl::DeleteTextures(1, &fb.image);
                        gl::GenTextures(1, &mut fb.image);
                        gl::BindTexture(gl::TEXTURE_2D, fb.image);

                        fb.mip_levels = size.calculate_miplevels();
                        if fb.mip_levels > fb.max_levels {
                            fb.mip_levels = fb.max_levels;
                        }
                        if fb.mip_levels == 0 {
                            fb.mip_levels = 1;
                        }

                        gl::TexStorage2D(
                            gl::TEXTURE_2D,
                            fb.mip_levels as GLsizei,
                            ImageFormat::R8G8B8A8Unorm.into(),
                            size.width as GLsizei,
                            size.height as GLsizei,
                        );
                        gl::FramebufferTexture2D(
                            gl::FRAMEBUFFER,
                            gl::COLOR_ATTACHMENT0,
                            gl::TEXTURE_2D,
                            fb.image,
                            0,
                        );
                        // fb.init =
                        //     gl::CheckFramebufferStatus(gl::FRAMEBUFFER) == gl::FRAMEBUFFER_COMPLETE;
                    }
                    _ => return Err(FilterChainError::FramebufferInit(status)),
                }
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        Ok(())
    }
}
