use crate::error;
use windows::core::PCSTR;
use windows::Win32::Graphics::Direct3D::Fxc::{
    D3DCompile, D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION,
};
use windows::Win32::Graphics::Direct3D::ID3DBlob;
use windows::Win32::Graphics::Direct3D12::{
    ID3D12Device, D3D12_FEATURE_DATA_FORMAT_SUPPORT, D3D12_FEATURE_FORMAT_SUPPORT,
};
use windows::Win32::Graphics::Dxgi::Common::*;

/// wtf retroarch?
const DXGI_FORMAT_EX_A4R4G4B4_UNORM: DXGI_FORMAT = DXGI_FORMAT(1000);

const fn d3d12_format_fallback_list(format: DXGI_FORMAT) -> Option<&'static [DXGI_FORMAT]> {
    match format {
        DXGI_FORMAT_R32G32B32A32_FLOAT => Some(&[
            DXGI_FORMAT_R32G32B32A32_FLOAT,
            DXGI_FORMAT_R16G16B16A16_FLOAT,
            DXGI_FORMAT_R32G32B32_FLOAT,
            DXGI_FORMAT_R11G11B10_FLOAT,
            DXGI_FORMAT_UNKNOWN,
        ]),
        DXGI_FORMAT_R16G16B16A16_FLOAT => Some(&[
            DXGI_FORMAT_R16G16B16A16_FLOAT,
            DXGI_FORMAT_R32G32B32A32_FLOAT,
            DXGI_FORMAT_R32G32B32_FLOAT,
            DXGI_FORMAT_R11G11B10_FLOAT,
            DXGI_FORMAT_UNKNOWN,
        ]),
        DXGI_FORMAT_R8G8B8A8_UNORM => Some(&[
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_FORMAT_B8G8R8X8_UNORM,
            DXGI_FORMAT_UNKNOWN,
        ]),
        DXGI_FORMAT_R8G8B8A8_UNORM_SRGB => Some(&[
            DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_FORMAT_B8G8R8X8_UNORM,
            DXGI_FORMAT_UNKNOWN,
        ]),
        DXGI_FORMAT_B8G8R8A8_UNORM => Some(&[
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_UNKNOWN,
        ]),
        DXGI_FORMAT_B8G8R8X8_UNORM => Some(&[
            DXGI_FORMAT_B8G8R8X8_UNORM,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_UNKNOWN,
        ]),
        DXGI_FORMAT_B5G6R5_UNORM => Some(&[
            DXGI_FORMAT_B5G6R5_UNORM,
            DXGI_FORMAT_B8G8R8X8_UNORM,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_UNKNOWN,
        ]),
        DXGI_FORMAT_EX_A4R4G4B4_UNORM | DXGI_FORMAT_B4G4R4A4_UNORM => Some(&[
            DXGI_FORMAT_B4G4R4A4_UNORM,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_UNKNOWN,
        ]),
        DXGI_FORMAT_A8_UNORM => Some(&[
            DXGI_FORMAT_A8_UNORM,
            DXGI_FORMAT_R8_UNORM,
            DXGI_FORMAT_R8G8_UNORM,
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_FORMAT_UNKNOWN,
        ]),
        DXGI_FORMAT_R8_UNORM => Some(&[
            DXGI_FORMAT_R8_UNORM,
            DXGI_FORMAT_A8_UNORM,
            DXGI_FORMAT_R8G8_UNORM,
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_FORMAT_UNKNOWN,
        ]),
        _ => None,
    }
}

pub fn d3d12_get_closest_format(
    device: &ID3D12Device,
    format: DXGI_FORMAT,
    format_support: D3D12_FEATURE_DATA_FORMAT_SUPPORT,
) -> DXGI_FORMAT {
    let default_list = [format, DXGI_FORMAT_UNKNOWN];
    let format_support_list = d3d12_format_fallback_list(format).unwrap_or(&default_list);

    for supported in format_support_list {
        unsafe {
            let mut support = D3D12_FEATURE_DATA_FORMAT_SUPPORT {
                Format: format,
                ..Default::default()
            };
            if device
                .CheckFeatureSupport(
                    D3D12_FEATURE_FORMAT_SUPPORT,
                    &mut support as *mut _ as *mut _,
                    std::mem::size_of::<D3D12_FEATURE_DATA_FORMAT_SUPPORT>() as u32,
                )
                .is_ok()
                && (support.Support1 & format_support.Support1) == format_support.Support1
                && (support.Support2 & format_support.Support2) == format_support.Support2
            {
                return *supported;
            }
        }
    }
    DXGI_FORMAT_UNKNOWN
}

pub fn d3d_compile_shader(source: &[u8], entry: &[u8], version: &[u8]) -> error::Result<ID3DBlob> {
    unsafe {
        let mut blob = None;
        D3DCompile(
            source.as_ptr().cast(),
            source.len(),
            None,
            None,
            None,
            PCSTR(entry.as_ptr()),
            PCSTR(version.as_ptr()),
            if cfg!(feature = "debug-shader") {
                D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
            } else {
                0
            },
            0,
            &mut blob,
            None,
        )?;

        Ok(blob.unwrap())
    }
}
