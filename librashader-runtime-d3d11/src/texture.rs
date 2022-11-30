use librashader_common::image::Image;
use librashader_common::{FilterMode, Size, WrapMode};
use windows::Win32::Graphics::Direct3D::D3D_SRV_DIMENSION_TEXTURE2D;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_FLAG,
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_CPU_ACCESS_FLAG,
    D3D11_CPU_ACCESS_WRITE, D3D11_FORMAT_SUPPORT_RENDER_TARGET, D3D11_FORMAT_SUPPORT_SHADER_SAMPLE,
    D3D11_FORMAT_SUPPORT_TEXTURE2D, D3D11_RESOURCE_MISC_FLAG, D3D11_RESOURCE_MISC_GENERATE_MIPS,
    D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA,
    D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DYNAMIC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;

use crate::util::Result;

#[derive(Debug, Clone)]
pub struct DxImageView {
    pub handle: ID3D11ShaderResourceView,
    pub size: Size<u32>, // pub image: GlImage,
}
#[derive(Debug, Clone)]
pub struct Texture {
    pub view: DxImageView,
    pub filter: FilterMode,
    pub wrap_mode: WrapMode,
    // pub mip_filter: FilterMode,
    // pub wrap_mode: WrapMode,
}

#[derive(Debug, Clone)]
pub struct OwnedTexture {
    pub handle: ID3D11Texture2D,
    // pub staging: ID3D11Texture2D,
    pub desc: D3D11_TEXTURE2D_DESC,
    pub image: Texture,
}

impl OwnedTexture {
    pub fn new(
        device: &ID3D11Device,
        source: &Image,
        desc: D3D11_TEXTURE2D_DESC,
        filter: FilterMode,
        wrap_mode: WrapMode,
    ) -> Result<OwnedTexture> {
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

        // determine if format is supported.
        // bruh why does  D3D11_FORMAT_SUPPORT not implement bitor???
        let mut format_support =
            D3D11_FORMAT_SUPPORT_TEXTURE2D.0 | D3D11_FORMAT_SUPPORT_SHADER_SAMPLE.0;
        if (desc.BindFlags & D3D11_BIND_RENDER_TARGET).0 != 0 {
            format_support |= D3D11_FORMAT_SUPPORT_RENDER_TARGET.0;
        }

        // eprintln!("s {:?}, p {:?}, l {:?}", source.size, source.pitch, source.bytes.len());
        // eprintln!("{:#?}", desc);

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

            let mut context = None;
            device.GetImmediateContext(&mut context);

            // todo: make this fallible
            let context = context.unwrap();

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

            Ok(OwnedTexture {
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
