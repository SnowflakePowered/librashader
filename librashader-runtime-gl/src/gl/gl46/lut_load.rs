use gl::types::{GLsizei, GLuint};
use rustc_hash::FxHashMap;
use librashader_common::image::Image;
use librashader_common::Size;
use librashader_presets::TextureConfig;
use crate::gl::LoadLut;
use crate::framebuffer::{GLImage, Viewport};
use crate::texture::Texture;
use crate::error::Result;

pub struct Gl46LutLoad;
impl LoadLut for Gl46LutLoad {
    fn load_luts(textures: &[TextureConfig]) -> Result<FxHashMap<usize, Texture>> {
        let mut luts = FxHashMap::default();
        let pixel_unpack = unsafe {
            let mut binding = 0;
            gl::GetIntegerv(gl::PIXEL_UNPACK_BUFFER_BINDING, &mut binding);
            binding
        };

        unsafe {
            gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0);
        }

        for (index, texture) in textures.iter().enumerate() {
            let image = Image::load(&texture.path)?;
            let levels = if texture.mipmap {
                librashader_runtime::scaling::calc_miplevel(image.size)
            } else {
                1u32
            };

            let mut handle = 0;
            unsafe {
                gl::CreateTextures(gl::TEXTURE_2D,1, &mut handle);

                gl::TextureStorage2D(
                    handle,
                    levels as GLsizei,
                    gl::RGBA8,
                    image.size.width as GLsizei,
                    image.size.height as GLsizei,
                );

                gl::PixelStorei(gl::UNPACK_ROW_LENGTH, 0);
                gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);

                gl::TextureSubImage2D(
                    handle,
                    0, 0, 0,
                    image.size.width as GLsizei,
                    image.size.height as GLsizei,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    image.bytes.as_ptr().cast(),
                );

                let mipmap = levels > 1;
                if mipmap {
                    gl::GenerateTextureMipmap(handle);
                }
            }

            luts.insert(
                index,
                Texture {
                    image: GLImage {
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
}