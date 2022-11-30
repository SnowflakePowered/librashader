use windows::Win32::Graphics::Direct3D11::{D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE, D3D11_FORMAT_SUPPORT_RENDER_TARGET, D3D11_FORMAT_SUPPORT_SHADER_SAMPLE, D3D11_FORMAT_SUPPORT_TEXTURE2D, D3D11_RENDER_TARGET_VIEW_DESC, D3D11_RENDER_TARGET_VIEW_DESC_0, D3D11_RTV_DIMENSION_TEXTURE2D, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_TEX2D_RTV, D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC, D3D11_VIEWPORT, ID3D11Device, ID3D11RenderTargetView, ID3D11ShaderResourceView, ID3D11Texture2D};
use windows::Win32::Graphics::Direct3D::D3D_SRV_DIMENSION_TEXTURE2D;
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};
use librashader_common::{ImageFormat, Size};
use librashader_presets::{Scale2D, ScaleType, Scaling};
use crate::texture::Texture;
use crate::util;
use crate::util::d3d11_get_closest_format;

#[derive(Debug, Clone)]
pub struct OwnedFramebuffer {
    pub texture: ID3D11Texture2D,
    pub size: Size<u32>,
    pub format: DXGI_FORMAT,
    device: ID3D11Device,
    is_raw: bool
}

impl OwnedFramebuffer {
    pub fn new(device: &ID3D11Device, size: Size<u32>, format: ImageFormat) -> util::Result<OwnedFramebuffer> {
        unsafe {
            let format = d3d11_get_closest_format(device, DXGI_FORMAT::from(format),
                                                  D3D11_FORMAT_SUPPORT_TEXTURE2D.0 | D3D11_FORMAT_SUPPORT_SHADER_SAMPLE.0 | D3D11_FORMAT_SUPPORT_RENDER_TARGET.0);
            eprintln!("{format:?}");
            let desc = default_desc(size, format);
            let texture = device.CreateTexture2D(&desc, None)?;

            Ok(OwnedFramebuffer {
                texture,
                size,
                format,
                device: device.clone(),
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
        source: &Texture,
    ) -> util::Result<Size<u32>> {
        if self.is_raw {
            return Ok(self.size);
        }

        let width;
        let height;

        match scaling.x {
            Scaling {
                scale_type: ScaleType::Input,
                factor,
            } => width = source.view.size.width * factor,
            Scaling {
                scale_type: ScaleType::Absolute,
                factor,
            } => width = factor.into(),
            Scaling {
                scale_type: ScaleType::Viewport,
                factor,
            } => width = viewport_size.width * factor,
        };

        match scaling.y {
            Scaling {
                scale_type: ScaleType::Input,
                factor,
            } => height = source.view.size.height * factor,
            Scaling {
                scale_type: ScaleType::Absolute,
                factor,
            } => height = factor.into(),
            Scaling {
                scale_type: ScaleType::Viewport,
                factor,
            } => height = viewport_size.height * factor,
        };

        let size = Size {
            width: width.round() as u32,
            height: height.round() as u32,
        };

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

    pub fn init(&mut self, size: Size<u32>, format: ImageFormat) -> util::Result<()> {
        if self.is_raw {
            return Ok(());
        }

        let format = d3d11_get_closest_format(&self.device, DXGI_FORMAT::from(format),
                                              D3D11_FORMAT_SUPPORT_TEXTURE2D.0 |
                                                  D3D11_FORMAT_SUPPORT_SHADER_SAMPLE.0 | D3D11_FORMAT_SUPPORT_RENDER_TARGET.0);

        let desc = default_desc(size, format);
        unsafe {
            let mut texture = self.device.CreateTexture2D(&desc, None)?;
            std::mem::swap(&mut self.texture, &mut texture);
            drop(texture)
        }
        self.format = format;

        Ok(())
    }

    pub fn create_shader_resource_view(&self) -> util::Result<ID3D11ShaderResourceView> {
        unsafe {
            Ok(self.device.CreateShaderResourceView(&self.texture, Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: self.format,
                ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: u32::MAX,
                    }
                },
            }))?)
        }
    }

    pub fn create_render_target_view(&self) -> util::Result<ID3D11RenderTargetView> {
        unsafe {
            Ok(self.device.CreateRenderTargetView(&self.texture, Some(&D3D11_RENDER_TARGET_VIEW_DESC {
                Format: self.format,
                ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_RENDER_TARGET_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_RTV {
                        MipSlice: 0,
                    }
                },
            }))?)
        }
    }

    pub fn as_output_framebuffer(&self) -> util::Result<OutputFramebuffer> {
        Ok(OutputFramebuffer {
            rtv: self.create_render_target_view()?,
            size: self.size,
            viewport: default_viewport(self.size)
        })
    }
}
#[derive(Debug, Clone)]
pub struct OutputFramebuffer {
    pub rtv: ID3D11RenderTargetView,
    pub size: Size<u32>,
    pub viewport: D3D11_VIEWPORT
}

fn default_desc(size: Size<u32>, format: DXGI_FORMAT) -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
        Width: size.width,
        Height: size.height,
        MipLevels: 1,
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