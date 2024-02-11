use crate::error::{FilterChainError, Result};
use crate::samplers::SamplerSet;
use crate::texture::MetalTexture;
use icrate::Metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLDevice, MTLOrigin,
    MTLPixelFormatBGRA8Unorm, MTLRegion, MTLSize, MTLTexture, MTLTextureDescriptor,
    MTLTextureUsageShaderRead,
};
use librashader_presets::TextureConfig;
use librashader_runtime::image::{Image, BGRA8};
use librashader_runtime::scaling::MipmapSize;
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use std::ffi::c_void;
use std::ptr::NonNull;

pub(crate) struct LutTexture(MetalTexture);

impl LutTexture {
    pub fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        image: Image<BGRA8>,
        cmd: &ProtocolObject<dyn MTLCommandBuffer>,
        config: &TextureConfig,
    ) -> Result<Self> {
        let descriptor = unsafe {
            let descriptor =
                MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                    MTLPixelFormatBGRA8Unorm,
                    image.size.width as usize,
                    image.size.height as usize,
                    config.mipmap,
                );

            descriptor.setSampleCount(1);
            descriptor.setMipmapLevelCount(if config.mipmap {
                image.size.calculate_miplevels() as usize
            } else {
                1
            });

            descriptor.setUsage(MTLTextureUsageShaderRead);

            descriptor
        };

        let texture = device
            .newTextureWithDescriptor(&descriptor)
            .ok_or(FilterChainError::FailedToCreateTexture)?;

        unsafe {
            let region = MTLRegion {
                origin: MTLOrigin { x: 0, y: 0, z: 0 },
                size: MTLSize {
                    width: image.size.width as usize,
                    height: image.size.height as usize,
                    depth: 1,
                },
            };

            texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                region,
                0,
                // SAFETY: replaceRegion withBytes is const.
                NonNull::new_unchecked(image.bytes.as_slice().as_ptr() as *mut c_void),
                4 * image.size.width as usize,
            )
        }

        if config.mipmap {
            if let Some(encoder) = cmd.blitCommandEncoder() {
                encoder.generateMipmapsForTexture(&texture);
                encoder.endEncoding();
            }
        }

        Ok(LutTexture(texture))
    }
}
