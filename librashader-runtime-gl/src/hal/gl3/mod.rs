use gl::types::{GLsizei, GLuint};
use rustc_hash::FxHashMap;
use librashader_common::image::Image;
use librashader_common::Size;
use librashader_presets::TextureConfig;
use crate::{GlImage, util};
use crate::hal::OpenGlAbstraction;
use crate::texture::Texture;

pub struct OpenGl3;

impl OpenGlAbstraction for OpenGl3 {
    fn load_luts(textures: &[TextureConfig]) -> crate::error::Result<FxHashMap<usize, Texture>> {
        let mut luts = FxHashMap::default();
        let pixel_unpack = unsafe {
            let mut binding = 0;
            gl::GetIntegerv(gl::PIXEL_UNPACK_BUFFER_BINDING, &mut binding);
            binding
        };

        for (index, texture) in textures.iter().enumerate() {
            let image = Image::load(&texture.path)?;
            let levels = if texture.mipmap {
                util::calc_miplevel(image.size)
            } else {
                1u32
            };

            let mut handle = 0;
            unsafe {
                gl::GenTextures(1, &mut handle);
                gl::BindTexture(gl::TEXTURE_2D, handle);
                gl::TexStorage2D(
                    gl::TEXTURE_2D,
                    levels as GLsizei,
                    gl::RGBA8,
                    image.size.width as GLsizei,
                    image.size.height as GLsizei,
                );

                gl::PixelStorei(gl::UNPACK_ROW_LENGTH, 0);
                gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);
                gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0);
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    image.size.width as GLsizei,
                    image.size.height as GLsizei,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    image.bytes.as_ptr().cast(),
                );

                let mipmap = levels > 1;
                if mipmap {
                    gl::GenerateMipmap(gl::TEXTURE_2D);
                }

                gl::BindTexture(gl::TEXTURE_2D, 0);
            }

            luts.insert(
                index,
                Texture {
                    image: GlImage {
                        handle,
                        format: gl::RGBA8,
                        size: image.size,
                        padded_size: Size::default(),
                    },
                    filter: texture.filter_mode,
                    mip_filter: texture.filter_mode,
                    wrap_mode: texture.wrap_mode,
                },
            );
        }

        unsafe {
            gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, pixel_unpack as GLuint);
        };
        Ok(luts)
    }

    fn clear_framebuffer<const REBIND: bool>(fbo: GLuint) {
        unsafe {
            if REBIND {
                gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            }
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            if REBIND {
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            }
        }
    }

    fn create_framebuffer() -> GLuint {
        let mut framebuffer = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
        framebuffer
    }

    fn copy_framebuffer() -> GLuint {
        todo!()
    }
}