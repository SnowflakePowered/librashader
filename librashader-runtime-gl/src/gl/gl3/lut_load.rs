use crate::error::{FilterChainError, Result};
use crate::framebuffer::GLImage;
use crate::gl::LoadLut;
use crate::texture::InputTexture;
use glow::{HasContext, PixelUnpackData};
use librashader_common::map::FastHashMap;
use librashader_presets::TextureConfig;
use librashader_runtime::image::{Image, ImageError, UVDirection};
use librashader_runtime::scaling::MipmapSize;
use rayon::prelude::*;
use std::num::NonZeroU32;

pub struct Gl3LutLoad;
impl LoadLut for Gl3LutLoad {
    fn load_luts(
        ctx: &glow::Context,
        textures: &[TextureConfig],
    ) -> Result<FastHashMap<usize, InputTexture>> {
        let mut luts = FastHashMap::default();
        let pixel_unpack = unsafe { ctx.get_parameter_i32(glow::PIXEL_UNPACK_BUFFER_BINDING) };

        let images = textures
            .par_iter()
            .map(|texture| Image::load(&texture.path, UVDirection::TopLeft))
            .collect::<std::result::Result<Vec<Image>, ImageError>>()?;

        for (index, (texture, image)) in textures.iter().zip(images).enumerate() {
            let levels = if texture.mipmap {
                image.size.calculate_miplevels()
            } else {
                1u32
            };

            let handle = unsafe {
                let handle = ctx.create_texture().map_err(FilterChainError::GlError)?;

                ctx.bind_texture(gl::TEXTURE_2D, Some(handle));
                ctx.tex_storage_2d(
                    glow::TEXTURE_2D,
                    levels as i32,
                    glow::RGBA8,
                    image.size.width as i32,
                    image.size.height as i32,
                );

                ctx.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
                ctx.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
                ctx.bind_buffer(gl::PIXEL_UNPACK_BUFFER, None);

                ctx.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    image.size.width as i32,
                    image.size.height as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    PixelUnpackData::Slice(&image.bytes),
                );

                let mipmap = levels > 1;
                if mipmap {
                    ctx.generate_mipmap(gl::TEXTURE_2D);
                }

                ctx.bind_texture(gl::TEXTURE_2D, None);
                handle
            };

            luts.insert(
                index,
                InputTexture {
                    image: GLImage {
                        handle: Some(handle),
                        format: gl::RGBA8,
                        size: image.size,
                    },
                    filter: texture.filter_mode,
                    mip_filter: texture.filter_mode,
                    wrap_mode: texture.wrap_mode,
                },
            );
        }

        unsafe {
            // todo: webgl doesn't support this.
            let pixel_unpack = NonZeroU32::try_from(pixel_unpack as u32)
                .ok()
                .map(glow::NativeBuffer);

            ctx.bind_buffer(gl::PIXEL_UNPACK_BUFFER, pixel_unpack);
        };
        Ok(luts)
    }
}
