use windows::Win32::Graphics::Direct3D11::{D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE, D3D11_FORMAT_SUPPORT_RENDER_TARGET, D3D11_FORMAT_SUPPORT_SHADER_SAMPLE, D3D11_FORMAT_SUPPORT_TEXTURE2D, D3D11_RESOURCE_MISC_GENERATE_MIPS, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DYNAMIC, ID3D11Device, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D};
use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;
use librashader_common::{FilterMode, Size, WrapMode};

#[derive(Debug, Clone)]
pub struct Texture {
    pub handle: ID3D11Texture2D,
    pub staging: ID3D11Texture2D,
    pub srv: ID3D11ShaderResourceView,
    pub sampler: ID3D11SamplerState,
    pub desc: D3D11_TEXTURE2D_DESC,
    pub size: Size<u32>
    // pub image: GlImage,
    // pub filter: FilterMode,
    // pub mip_filter: FilterMode,
    // pub wrap_mode: WrapMode,
}

impl Texture {
    pub fn new(device: &ID3D11Device, size: Size<u32>, desc: D3D11_TEXTURE2D_DESC) -> Texture {
        let mut desc = D3D11_TEXTURE2D_DESC {
            Width: size.width,
            Height: size.height,
            MipLevels: 1,
            ArraySize: 1,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0
            },
            CPUAccessFlags: if desc.Usage == D3D11_USAGE_DYNAMIC {
                D3D11_CPU_ACCESS_WRITE
            } else {
                0
            },
            ..desc
        };
        desc.BindFlags |= D3D11_BIND_SHADER_RESOURCE;

        // determine number of mipmaps required
        if desc.MiscFlags & D3D11_RESOURCE_MISC_GENERATE_MIPS {
            let mut width = desc.Width >> 5;
            let mut height = desc.Height >> 5;
            desc.BindFlags |= D3D11_BIND_RENDER_TARGET;

            while width != 0 && height != 0 {
                width  >>= 1;
                height >>= 1;
                desc.MipLevels += 1;
            }
        }

        // determine if format is supported.
        let mut format_support = D3D11_FORMAT_SUPPORT_TEXTURE2D | D3D11_FORMAT_SUPPORT_SHADER_SAMPLE;
        if desc.BindFlags |= D3D11_BIND_RENDER_TARGET {
            format_support |= D3D11_FORMAT_SUPPORT_RENDER_TARGET;
        }

        // todo: actually check format support

        // d3d11_common: 83
        todo!();

    }
}