use crate::error::{FilterChainError, Result};
use crate::framebuffer::GLImage;
use crate::gl::LoadLut;
use crate::texture::InputTexture;
use glow::{HasContext, PixelUnpackData};
use librashader_common::map::FastHashMap;
use librashader_pack::TextureResource;
use librashader_runtime::image::{ImageError, LoadedTexture, UVDirection};
use librashader_runtime::scaling::MipmapSize;
use rayon::prelude::*;

pub struct Gl46LutLoad;
impl LoadLut for Gl46LutLoad {
    fn load_luts(
        context: &glow::Context,
        textures: Vec<TextureResource>,
    ) -> Result<FastHashMap<usize, InputTexture>> {
        let mut luts = FastHashMap::default();

        // don't need this for texture DSA api.

        let textures = textures
            .into_par_iter()
            .map(|texture| LoadedTexture::from_texture(texture, UVDirection::TopLeft))
            .collect::<std::result::Result<Vec<LoadedTexture>, ImageError>>()?;

        for (index, LoadedTexture { meta, image }) in textures.iter().enumerate() {
            let levels = if meta.mipmap {
                image.size.calculate_miplevels()
            } else {
                1u32
            };

            let handle = unsafe {
                let handle = context
                    .create_named_texture(glow::TEXTURE_2D)
                    .map_err(FilterChainError::GlError)?;

                context.texture_storage_2d(
                    handle,
                    levels as i32,
                    glow::RGBA8,
                    image.size.width as i32,
                    image.size.height as i32,
                );

                context.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
                context.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);

                context.texture_sub_image_2d(
                    handle,
                    0,
                    0,
                    0,
                    image.size.width as i32,
                    image.size.height as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    PixelUnpackData::Slice(Some(&image.bytes)),
                );

                let mipmap = levels > 1;
                if mipmap {
                    context.generate_texture_mipmap(handle);
                }

                handle
            };

            luts.insert(
                index,
                InputTexture {
                    image: GLImage {
                        handle: Some(handle),
                        format: glow::RGBA8,
                        size: image.size,
                    },
                    filter: meta.filter_mode,
                    mip_filter: meta.filter_mode,
                    wrap_mode: meta.wrap_mode,
                },
            );
        }

        // unsafe {
        //     context.bind_buffer(glow::PIXEL_UNPACK_BUFFER, pixel_unpack);
        // };
        Ok(luts)
    }
}
