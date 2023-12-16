use std::sync::Arc;
use wgpu::TextureFormat;
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::Scale2D;
use librashader_runtime::scaling::{MipmapSize, ViewportSize};

pub struct OwnedImage {
    device: Arc<wgpu::Device>,
    pub image: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub max_miplevels: u32,
    pub levels: u32,
    pub size: Size<u32>,
    pub format: wgpu::TextureFormat,
}

pub struct InputImage {
    /// A handle to the `VkImage`.
    pub image: wgpu::Texture,
    pub image_view: wgpu::TextureView,
    pub wrap_mode: WrapMode,
    pub filter_mode: FilterMode,
    pub mip_filter: FilterMode,
}


impl OwnedImage {
    pub fn new(device: Arc<wgpu::Device>,
               size: Size<u32>,
               max_miplevels: u32,
        format: ImageFormat,
    ) -> Self {

        let format: Option<wgpu::TextureFormat> = format.into();
        let format = format.unwrap_or(wgpu::TextureFormat::Rgba8Unorm);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: size.into(),
            mip_level_count: std::cmp::min(max_miplevels, size.calculate_miplevels()),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[format.into()],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        Self {
            device,
            image: texture,
            view,
            max_miplevels,
            levels: std::cmp::min(max_miplevels, size.calculate_miplevels()),
            size,
            format,
        }
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
        let format: Option<wgpu::TextureFormat> = format.into();
        let format = format.unwrap_or(wgpu::TextureFormat::Rgba8Unorm);

        if self.size != size
            || (mipmap && self.max_miplevels == 1)
            || (!mipmap && self.max_miplevels != 1)
            || format != self.format
        {
            let mut new = OwnedImage::new(Arc::clone(&self.device), size, self.max_miplevels, format.into());
            std::mem::swap(self, &mut new);
        }
        size
    }
}