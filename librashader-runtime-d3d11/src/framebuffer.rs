use crate::error;
use crate::texture::{DxImageView, Texture};
use crate::util::d3d11_get_closest_format;
use librashader_common::{ImageFormat, Size};
use librashader_presets::Scale2D;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::D3D_SRV_DIMENSION_TEXTURE2D;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11RenderTargetView, ID3D11ShaderResourceView,
    ID3D11Texture2D, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE,
    D3D11_FORMAT_SUPPORT_RENDER_TARGET, D3D11_FORMAT_SUPPORT_SHADER_SAMPLE,
    D3D11_FORMAT_SUPPORT_TEXTURE2D, D3D11_RENDER_TARGET_VIEW_DESC, D3D11_RENDER_TARGET_VIEW_DESC_0,
    D3D11_RTV_DIMENSION_TEXTURE2D, D3D11_SHADER_RESOURCE_VIEW_DESC,
    D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_TEX2D_RTV, D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, D3D11_VIEWPORT,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};

#[derive(Debug, Clone)]
pub(crate) struct OwnedFramebuffer {
    pub texture: ID3D11Texture2D,
    pub size: Size<u32>,
    pub format: DXGI_FORMAT,
    pub max_levels: u32,
    device: ID3D11Device,
    is_raw: bool,
}

impl OwnedFramebuffer {
    pub fn new(
        device: &ID3D11Device,
        size: Size<u32>,
        mip_levels: u32,
        format: ImageFormat,
    ) -> error::Result<OwnedFramebuffer> {
        unsafe {
            let format = d3d11_get_closest_format(
                device,
                DXGI_FORMAT::from(format),
                D3D11_FORMAT_SUPPORT_TEXTURE2D.0
                    | D3D11_FORMAT_SUPPORT_SHADER_SAMPLE.0
                    | D3D11_FORMAT_SUPPORT_RENDER_TARGET.0,
            );
            let desc = default_desc(size, format, mip_levels);
            let texture = device.CreateTexture2D(&desc, None)?;

            Ok(OwnedFramebuffer {
                texture,
                size,
                format,
                device: device.clone(),
                max_levels: mip_levels,
                is_raw: false,
            })
        }
    }


    pub(crate) fn scale(
        &mut self,
        scaling: Scale2D,
        format: ImageFormat,
        viewport_size: &Size<u32>,
        _original: &Texture,
        source: &Texture
    ) -> error::Result<Size<u32>> {
        if self.is_raw {
            return Ok(self.size);
        }

        let size = librashader_runtime::scaling::scale(scaling, source.view.size, *viewport_size);

        if self.size != size {
            self.size = size;

            self.init(
                size,
                if format == ImageFormat::Unknown {
                    ImageFormat::R8G8B8A8Unorm
                } else {
                    format
                },
            )?;
        }
        Ok(size)
    }

    pub fn init(&mut self, size: Size<u32>, format: ImageFormat) -> error::Result<()> {
        if self.is_raw {
            return Ok(());
        }

        let format = d3d11_get_closest_format(
            &self.device,
            DXGI_FORMAT::from(format),
            D3D11_FORMAT_SUPPORT_TEXTURE2D.0
                | D3D11_FORMAT_SUPPORT_SHADER_SAMPLE.0
                | D3D11_FORMAT_SUPPORT_RENDER_TARGET.0,
        );

        let desc = default_desc(size, format, self.max_levels);
        unsafe {
            let mut texture = self.device.CreateTexture2D(&desc, None)?;
            std::mem::swap(&mut self.texture, &mut texture);
            drop(texture)
        }
        self.format = format;
        self.size = size;

        Ok(())
    }

    pub fn create_shader_resource_view(&self) -> error::Result<ID3D11ShaderResourceView> {
        unsafe {
            Ok(self.device.CreateShaderResourceView(
                &self.texture,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: self.format,
                    ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: u32::MAX,
                        },
                    },
                }),
            )?)
        }
    }

    pub fn create_render_target_view(&self) -> error::Result<ID3D11RenderTargetView> {
        unsafe {
            Ok(self.device.CreateRenderTargetView(
                &self.texture,
                Some(&D3D11_RENDER_TARGET_VIEW_DESC {
                    Format: self.format,
                    ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_RENDER_TARGET_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 },
                    },
                }),
            )?)
        }
    }

    pub fn as_output_framebuffer(&self) -> error::Result<OutputFramebuffer> {
        Ok(OutputFramebuffer {
            rtv: self.create_render_target_view()?,
            size: self.size,
            viewport: default_viewport(self.size),
        })
    }

    pub fn copy_from(&mut self, image: &DxImageView) -> error::Result<()> {
        let resource: ID3D11Texture2D = unsafe {
            let mut resource = None;
            image.handle.GetResource(&mut resource);
            let Some(resource) = resource else {
                return Ok(())
            };
            resource.cast()?
        };

        let format = unsafe {
            let mut desc = Default::default();
            resource.GetDesc(&mut desc);
            desc.Format
        };

        if self.size != image.size || format != self.format {
            eprintln!("[history] resizing");
            self.init(image.size, ImageFormat::from(format))?;
        }

        self.texture = resource;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub(crate) struct OutputFramebuffer {
    pub rtv: ID3D11RenderTargetView,
    pub size: Size<u32>,
    pub viewport: D3D11_VIEWPORT,
}

fn default_desc(size: Size<u32>, format: DXGI_FORMAT, mip_levels: u32) -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
        Width: size.width,
        Height: size.height,
        MipLevels: mip_levels,
        ArraySize: 1,
        Format: format,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET,
        CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
        MiscFlags: Default::default(),
    }
}
pub const fn default_viewport(size: Size<u32>) -> D3D11_VIEWPORT {
    D3D11_VIEWPORT {
        TopLeftX: 0.0,
        TopLeftY: 0.0,
        Width: size.width as f32,
        Height: size.height as f32,
        MinDepth: 0.0,
        MaxDepth: 1.0,
    }
}
