use crate::render::RenderTest;
use anyhow::anyhow;
use image::RgbaImage;
use librashader::runtime::wgpu::*;
use librashader::runtime::Viewport;
use librashader_runtime::image::{Image, UVDirection};
use std::io::{Cursor, Write};
use std::ops::DerefMut;
use std::path::Path;
use std::sync::Arc;
use wgpu::{Adapter, Device, Instance, Queue, Texture};
use wgpu_types::{
    BufferAddress, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, ImageCopyBuffer,
    ImageDataLayout, Maintain, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

use parking_lot::Mutex;

pub struct Wgpu {
    instance: Instance,
    adapter: Adapter,
    device: Arc<Device>,
    queue: Arc<Queue>,
    image: Image,
    texture: Arc<Texture>,
}

struct BufferDimensions {
    width: usize,
    height: usize,
    unpadded_bytes_per_row: usize,
    padded_bytes_per_row: usize,
}

impl BufferDimensions {
    fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }
}

impl RenderTest for Wgpu {
    fn new(path: impl AsRef<Path>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Wgpu::new(path)
    }

    fn render(&mut self, path: impl AsRef<Path>, frame_count: usize) -> anyhow::Result<RgbaImage> {
        let mut chain = FilterChain::load_from_path(
            path,
            Arc::clone(&self.device),
            Arc::clone(&self.queue),
            Some(&FilterChainOptions {
                force_no_mipmaps: false,
                enable_cache: true,
                adapter_info: None,
            }),
        )?;

        let mut cmd = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        let output_tex = self.device.create_texture(&TextureDescriptor {
            label: None,
            size: self.texture.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });

        let buffer_dimensions =
            BufferDimensions::new(output_tex.width() as usize, output_tex.height() as usize);
        let output_buf = Arc::new(self.device.create_buffer(&BufferDescriptor {
            label: None,
            size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height)
                as BufferAddress, // 4bpp
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }));

        let view = output_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let output = WgpuOutputView::new_from_raw(
            &view,
            output_tex.size().into(),
            TextureFormat::Rgba8Unorm,
        );

        chain.frame(
            Arc::clone(&self.texture),
            &Viewport::new_render_target_sized_origin(output, None)?,
            &mut cmd,
            frame_count,
            None,
        )?;

        cmd.copy_texture_to_buffer(
            output_tex.as_image_copy(),
            ImageCopyBuffer {
                buffer: &output_buf,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(buffer_dimensions.padded_bytes_per_row as u32),
                    rows_per_image: None,
                },
            },
            output_tex.size(),
        );

        let si = self.queue.submit([cmd.finish()]);
        self.device.poll(Maintain::WaitForSubmissionIndex(si));

        let capturable = Arc::clone(&output_buf);

        let mut pixels = Arc::new(Mutex::new(Vec::new()));

        let pixels_async = Arc::clone(&pixels);
        output_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                if r.is_ok() {
                    let buffer = capturable.slice(..).get_mapped_range();
                    let mut pixels = pixels_async.lock();
                    pixels.resize(buffer.len(), 0);

                    let mut cursor = Cursor::new(pixels.deref_mut());
                    for chunk in buffer.chunks(buffer_dimensions.padded_bytes_per_row) {
                        cursor
                            .write_all(&chunk[..buffer_dimensions.unpadded_bytes_per_row])
                            .unwrap()
                    }

                    cursor.into_inner();
                }
                capturable.unmap();
            });

        self.device.poll(Maintain::Wait);

        if pixels.lock().len() == 0 {
            return Err(anyhow!("failed to copy pixels from buffer"));
        }

        let image = RgbaImage::from_raw(
            output_tex.width(),
            output_tex.height(),
            pixels.lock().to_vec(),
        )
        .ok_or(anyhow!("Unable to create image from data"))?;

        Ok(image)
    }
}

impl Wgpu {
    pub fn new(image: impl AsRef<Path>) -> anyhow::Result<Self> {
        pollster::block_on(async {
            let instance = wgpu::Instance::default();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .ok_or(anyhow!("Couldn't request WGPU adapter"))?;

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        required_features: wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
                            | wgpu::Features::PIPELINE_CACHE
                            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                            | wgpu::Features::FLOAT32_FILTERABLE,
                        required_limits: wgpu::Limits::default(),
                        label: None,
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await?;
            let (image, texture) = Self::load_image(&device, &queue, image)?;

            Ok(Self {
                instance,
                adapter,
                device: Arc::new(device),
                queue: Arc::new(queue),
                image,
                texture: Arc::new(texture),
            })
        })
    }

    fn load_image(
        device: &Device,
        queue: &Queue,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<(Image, Texture)> {
        let image = Image::load(path, UVDirection::TopLeft)?;
        let texture = device.create_texture(&TextureDescriptor {
            size: image.size.into(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            format: wgpu::TextureFormat::Rgba8Unorm,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
            label: None,
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
            image.size.into(),
        );

        let si = queue.submit([]);

        device.poll(Maintain::WaitForSubmissionIndex(si));

        Ok((image, texture))
    }
}
