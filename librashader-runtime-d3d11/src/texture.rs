use librashader_runtime::image::Image;
use librashader_common::{FilterMode, Size, WrapMode};
use windows::Win32::Graphics::Direct3D::D3D_SRV_DIMENSION_TEXTURE2D;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_FLAG,
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_CPU_ACCESS_FLAG,
    D3D11_CPU_ACCESS_WRITE, D3D11_RESOURCE_MISC_FLAG, D3D11_RESOURCE_MISC_GENERATE_MIPS,
    D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA,
    D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DYNAMIC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;

use crate::error::Result;
use crate::framebuffer::OwnedFramebuffer;

#[derive(Debug, Clone)]
pub struct DxImageView {
    pub handle: ID3D11ShaderResourceView,
    pub size: Size<u32>,
}
#[derive(Debug, Clone)]
pub(crate) struct Texture {
    pub view: DxImageView,
    pub filter: FilterMode,
    pub wrap_mode: WrapMode,
}

impl Texture {
    pub fn from_framebuffer(fbo: &OwnedFramebuffer, wrap_mode: WrapMode, filter: FilterMode) -> Result<Self> {
        Ok(Texture {
            view: DxImageView {
                handle: fbo.create_shader_resource_view()?,
                size: fbo.size,
            },
            filter,
            wrap_mode
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LutTexture {
    // The handle to the Texture2D must be kept alive.
    #[allow(dead_code)]
    pub handle: ID3D11Texture2D,
    #[allow(dead_code)]
    pub desc: D3D11_TEXTURE2D_DESC,
    pub image: Texture,
}

impl LutTexture {
    pub fn new(
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        source: &Image,
        desc: D3D11_TEXTURE2D_DESC,
        filter: FilterMode,
        wrap_mode: WrapMode,
    ) -> Result<LutTexture> {
        let mut desc = D3D11_TEXTURE2D_DESC {
            Width: source.size.width,
            Height: source.size.height,
            // todo: set this to 0
            MipLevels: if (desc.MiscFlags & D3D11_RESOURCE_MISC_GENERATE_MIPS).0 != 0 {
                0
            } else {
                1
            },
            ArraySize: 1,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            CPUAccessFlags: if desc.Usage == D3D11_USAGE_DYNAMIC {
                D3D11_CPU_ACCESS_WRITE
            } else {
                D3D11_CPU_ACCESS_FLAG(0)
            },
            ..desc
        };
        desc.BindFlags |= D3D11_BIND_SHADER_RESOURCE;

        // determine number of mipmaps required
        if (desc.MiscFlags & D3D11_RESOURCE_MISC_GENERATE_MIPS).0 != 0 {
            let mut width = desc.Width >> 5;
            let mut height = desc.Height >> 5;
            desc.BindFlags |= D3D11_BIND_RENDER_TARGET;

            while width != 0 && height != 0 {
                width >>= 1;
                height >>= 1;
                desc.MipLevels += 1;
            }
        }

        // Don't need to determine format support because LUTs are always DXGI_FORMAT_R8G8B8A8_UNORM
        // since we load them with the Image module.

        unsafe {
            let handle = device.CreateTexture2D(&desc, None).unwrap();

            let srv = device.CreateShaderResourceView(
                &handle,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: desc.Format,
                    ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: u32::MAX,
                        },
                    },
                }),
            )?;

            // need a staging texture to defer mipmap generation
            let staging = device.CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    MipLevels: 1,
                    BindFlags: D3D11_BIND_FLAG(0),
                    MiscFlags: D3D11_RESOURCE_MISC_FLAG(0),
                    Usage: D3D11_USAGE_STAGING,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                    ..desc
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: source.bytes.as_ptr().cast(),
                    SysMemPitch: source.pitch as u32,
                    SysMemSlicePitch: 0,
                }),
            )?;

            // todo: do format conversion (leverage image crate..?
            // is this necessary with CopySubresourceRegion)...

            context.CopySubresourceRegion(
                &handle,
                0,
                0,
                0,
                0,
                &staging,
                0,
                Some(&D3D11_BOX {
                    left: 0,
                    top: 0,
                    front: 0,
                    right: source.size.width,
                    bottom: source.size.height,
                    back: 1,
                }),
            );

            if (desc.MiscFlags & D3D11_RESOURCE_MISC_GENERATE_MIPS).0 != 0 {
                context.GenerateMips(&srv)
            }

            // let mut subresource = context.Map(staging, 0, D3D11_MAP_WRITE, 0)?;
            // staging.Upd

            Ok(LutTexture {
                handle,
                // staging,
                desc,
                image: Texture {
                    view: DxImageView {
                        handle: srv,
                        size: source.size,
                    },
                    filter,
                    wrap_mode,
                },
            })
        }
    }
}
