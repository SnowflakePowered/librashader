use std::sync::Arc;
use icrate::Metal::{MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLDevice, MTLPixelFormat, MTLPixelFormatBGRA8Unorm, MTLTexture, MTLTextureDescriptor, MTLTextureUsageRenderTarget, MTLTextureUsageShaderRead, MTLTextureUsageShaderWrite};
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::Scale2D;
use librashader_runtime::scaling::{MipmapSize, ViewportSize};
use crate::error::{Result, FilterChainError};

pub type MetalTexture = Id<ProtocolObject<dyn MTLTexture>>;

pub struct OwnedImage(MetalTexture);


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

            descriptor.setUsage(MTLTextureUsageShaderRead |  MTLTextureUsageRenderTarget);

            descriptor
        };

        Ok(Self(device.newTextureWithDescriptor(&descriptor)
            .ok_or(FilterChainError::FailedToCreateTexture)?))
    }

    pub fn scale(
        &mut self,
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
            || format != self.image.format()
        {
            let mut new = OwnedImage::new(
                Arc::clone(&self.device),
                size,
                self.max_miplevels,
                format.into(),
            );
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

    pub fn copy_from(&self, other: &ProtocolObject<dyn MTLTexture>, cmd: Id<ProtocolObject<dyn MTLCommandBuffer>>) -> Result<()> {
        let encoder = cmd.blitCommandEncoder()
            .ok_or(FilterChainError::FailedToCreateCommandBuffer)?;
        unsafe {
            encoder.copyFromTexture_toTexture(&self.0, other);
        }
        encoder.endEncoding();
        Ok(())
    }

    pub fn clear(&self, cmd: Id<ProtocolObject<dyn MTLCommandBuffer>>) {

        // cmd.clear_texture(&self.image, &wgpu::ImageSubresourceRange::default());
    }
    // pub fn generate_mipmaps(
    //     &self,
    //     cmd: &mut wgpu::CommandEncoder,
    //     mipmapper: &mut MipmapGen,
    //     sampler: &wgpu::Sampler,
    // ) {
    //     mipmapper.generate_mipmaps(cmd, &self.image, sampler, self.max_miplevels);
    // }
}
