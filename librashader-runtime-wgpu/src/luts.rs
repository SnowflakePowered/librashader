use wgpu::{ImageDataLayout, Label, TextureDescriptor};
use wgpu::util::DeviceExt;
use librashader_presets::TextureConfig;
use librashader_runtime::image::{BGRA8, Image};
use librashader_runtime::scaling::MipmapSize;

pub(crate) struct LutTexture {
    texture: wgpu::Texture,
}

impl LutTexture {
    pub fn new(
        device: &wgpu::Device,
        queue: &mut wgpu::Queue,
        _cmd: &mut wgpu::CommandEncoder,
        image: Image,
        config: &TextureConfig
    ) -> LutTexture {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(&config.name),
            size: image.size.into(),
            mip_level_count: if config.mipmap {
                image.size.calculate_miplevels()
            } else {
                1
            },
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                // need render attachment for mipmaps...
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });


        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.size.width),
                rows_per_image: None,
            },
            image.size.into()
        );

        // todo: mipmaps

        Self {
            texture
        }
    }
}