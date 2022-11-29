use crate::{FilterMode, ImageFormat, WrapMode};
use  windows::Win32::Graphics::Direct3D11;
use windows::Win32::Graphics::Dxgi::Common as dxgi;

impl From<ImageFormat> for dxgi::DXGI_FORMAT {
    fn from(format: ImageFormat) -> Self {
        match format {
            ImageFormat::Unknown => dxgi::DXGI_FORMAT_UNKNOWN,
            ImageFormat::R8Unorm => dxgi::DXGI_FORMAT_R8_UNORM,
            ImageFormat::R8Uint => dxgi::DXGI_FORMAT_R8_UINT,
            ImageFormat::R8Sint => dxgi::DXGI_FORMAT_R8_SINT,
            ImageFormat::R8G8Unorm => dxgi::DXGI_FORMAT_R8G8_UNORM,
            ImageFormat::R8G8Uint => dxgi::DXGI_FORMAT_R8G8_UINT,
            ImageFormat::R8G8Sint => dxgi::DXGI_FORMAT_R8G8_SINT,
            ImageFormat::R8G8B8A8Unorm => dxgi::DXGI_FORMAT_R8G8B8A8_UNORM,
            ImageFormat::R8G8B8A8Uint => dxgi::DXGI_FORMAT_R8G8B8A8_UINT,
            ImageFormat::R8G8B8A8Sint => dxgi::DXGI_FORMAT_R8G8B8A8_SINT,
            ImageFormat::R8G8B8A8Srgb => dxgi::DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
            ImageFormat::A2B10G10R10UnormPack32 => dxgi::DXGI_FORMAT_R10G10B10A2_UNORM,
            ImageFormat::A2B10G10R10UintPack32 => dxgi::DXGI_FORMAT_R10G10B10A2_UINT,
            ImageFormat::R16Uint => dxgi::DXGI_FORMAT_R16_UINT,
            ImageFormat::R16Sint => dxgi::DXGI_FORMAT_R16_SINT,
            ImageFormat::R16Sfloat => dxgi::DXGI_FORMAT_R16_FLOAT,
            ImageFormat::R16G16Uint => dxgi::DXGI_FORMAT_R16G16_UINT,
            ImageFormat::R16G16Sint => dxgi::DXGI_FORMAT_R16G16_SINT,
            ImageFormat::R16G16Sfloat => dxgi::DXGI_FORMAT_R16G16_FLOAT,
            ImageFormat::R16G16B16A16Uint => dxgi::DXGI_FORMAT_R16G16B16A16_UINT,
            ImageFormat::R16G16B16A16Sint => dxgi::DXGI_FORMAT_R16G16B16A16_SINT,
            ImageFormat::R16G16B16A16Sfloat => dxgi::DXGI_FORMAT_R16G16B16A16_FLOAT,
            ImageFormat::R32Uint => dxgi::DXGI_FORMAT_R32_UINT,
            ImageFormat::R32Sint =>dxgi::DXGI_FORMAT_R32_SINT,
            ImageFormat::R32Sfloat => dxgi::DXGI_FORMAT_R32_FLOAT,
            ImageFormat::R32G32Uint => dxgi::DXGI_FORMAT_R32G32_UINT,
            ImageFormat::R32G32Sint => dxgi::DXGI_FORMAT_R32G32_SINT,
            ImageFormat::R32G32Sfloat => dxgi::DXGI_FORMAT_R32G32_FLOAT,
            ImageFormat::R32G32B32A32Uint => dxgi::DXGI_FORMAT_R32G32B32A32_UINT,
            ImageFormat::R32G32B32A32Sint => dxgi::DXGI_FORMAT_R32G32B32A32_SINT,
            ImageFormat::R32G32B32A32Sfloat => dxgi::DXGI_FORMAT_R32G32B32A32_FLOAT,
        }
    }
}

impl From<WrapMode> for Direct3D11::D3D11_TEXTURE_ADDRESS_MODE {
    fn from(value: WrapMode) -> Self {
        match value {
            WrapMode::ClampToBorder => Direct3D11::D3D11_TEXTURE_ADDRESS_BORDER,
            WrapMode::ClampToEdge => Direct3D11::D3D11_TEXTURE_ADDRESS_CLAMP,
            WrapMode::Repeat => Direct3D11::D3D11_TEXTURE_ADDRESS_WRAP,
            WrapMode::MirroredRepeat => Direct3D11::D3D11_TEXTURE_ADDRESS_MIRROR,
        }
    }
}

impl From<FilterMode> for Direct3D11::D3D11_FILTER {
    fn from(value: FilterMode) -> Self {
        match value {
            FilterMode::Linear => Direct3D11::D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            _ => Direct3D11::D3D11_FILTER_MIN_MAG_MIP_POINT
        }
    }
}
