use crate::error::{FilterChainError, Result};
use icrate::Metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLDevice, MTLPixelFormat,
    MTLPixelFormatBGRA8Unorm, MTLTexture, MTLTextureDescriptor, MTLTextureUsageRenderTarget,
    MTLTextureUsageShaderRead, MTLTextureUsageShaderWrite,
};
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::Scale2D;
use librashader_runtime::scaling::{MipmapSize, ViewportSize};
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use std::sync::Arc;

pub type MetalTexture = Id<ProtocolObject<dyn MTLTexture>>;

pub struct OwnedImage {
    image: MetalTexture,
    max_miplevels: u32,
    size: Size<u32>,
}

impl OwnedImage {
    pub fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        size: Size<u32>,
        max_miplevels: u32,
        format: ImageFormat,
    ) -> Result<Self> {
        let format: MTLPixelFormat = format.into();

        let descriptor = unsafe {
            let descriptor =
                MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                    format,
                    size.width as usize,
                    size.height as usize,
                    max_miplevels <= 1,
                );

            descriptor.setSampleCount(1);
            descriptor.setMipmapLevelCount(if max_miplevels <= 1 {
                size.calculate_miplevels() as usize
            } else {
                1
            });

            descriptor.setUsage(
                MTLTextureUsageShaderRead
                    | MTLTextureUsageShaderWrite
                    | MTLTextureUsageRenderTarget,
            );

            descriptor
        };

        Ok(Self {
            image: device
                .newTextureWithDescriptor(&descriptor)
                .ok_or(FilterChainError::FailedToCreateTexture)?,
            max_miplevels,
            size,
        })
    }

    pub fn scale(
        &mut self,
        device: &ProtocolObject<dyn MTLDevice>,
        scaling: Scale2D,
        format: ImageFormat,
        viewport_size: &Size<u32>,
        source_size: &Size<u32>,
        mipmap: bool,
    ) -> Size<u32> {
        let size = source_size.scale_viewport(scaling, *viewport_size);
        let format: MTLPixelFormat = format.into();

        if self.size != size
            || (mipmap && self.max_miplevels == 1)
            || (!mipmap && self.max_miplevels != 1)
            || format != self.image.pixelFormat()
        {
            let mut new = OwnedImage::new(device, size, self.max_miplevels, format.into())?;
            std::mem::swap(self, &mut new);
        }
        size
    }

    // pub(crate) fn as_input(&self, filter: FilterMode, wrap_mode: WrapMode) -> InputImage {
    //     InputImage {
    //         image: Arc::clone(&self.image),
    //         view: Arc::clone(&self.view),
    //         wrap_mode,
    //         filter_mode: filter,
    //         mip_filter: filter,
    //     }
    // }

    pub fn copy_from(
        &self,
        other: &ProtocolObject<dyn MTLTexture>,
        cmd: Id<ProtocolObject<dyn MTLCommandBuffer>>,
    ) -> Result<()> {
        let encoder = cmd
            .blitCommandEncoder()
            .ok_or(FilterChainError::FailedToCreateCommandBuffer)?;
        unsafe {
            encoder.copyFromTexture_toTexture(other, &self.image);
        }
        encoder.generateMipmapsForTexture(&self.image);
        encoder.endEncoding();
        Ok(())
    }

    pub fn clear(&self, cmd: Id<ProtocolObject<dyn MTLCommandBuffer>>) {
        // let render = cmd.renderCommandEncoder()
        //     .ok_or(FilterChainError::FailedToCreateCommandBuffer)?;
        // render.
        // cmd.clear_texture(&self.image, &wgpu::ImageSubresourceRange::default());
    }

    /// caller must end the blit encoder after.
    pub fn generate_mipmaps(&self, mipmapper: &ProtocolObject<dyn MTLBlitCommandEncoder>) {
        mipmapper.generateMipmapsForTexture(&self.image);
    }
}
