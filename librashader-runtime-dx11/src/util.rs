use librashader_common::{FilterMode, Size, WrapMode};

use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use std::error::Error;

/// wtf retroarch?
const DXGI_FORMAT_EX_A4R4G4B4_UNORM: DXGI_FORMAT = DXGI_FORMAT(1000);

const fn d3d11_format_fallback_list(format: DXGI_FORMAT) -> Option<&'static [DXGI_FORMAT]> {
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

pub fn d3d11_get_closest_format(
    device: &ID3D11Device,
    format: DXGI_FORMAT,
    format_support_mask: i32,
) -> DXGI_FORMAT {
    let default_list = [format, DXGI_FORMAT_UNKNOWN];
    let format_support_list = d3d11_format_fallback_list(format)
        .unwrap_or(&default_list);
    let format_support_mask = format_support_mask as u32;

    for supported in format_support_list {
        unsafe {
            if let Ok(supported_format) = device.CheckFormatSupport(*supported)
                && (supported_format & format_support_mask) == format_support_mask {
                return *supported;
            }
        }
    }
    return DXGI_FORMAT_UNKNOWN;
}

// todo: d3d11.c 2097
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
