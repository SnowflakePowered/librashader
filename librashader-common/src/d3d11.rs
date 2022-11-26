use crate::{FilterMode, ShaderFormat, WrapMode};
use  windows::Win32::Graphics::Direct3D11;
use windows::Win32::Graphics::Dxgi::Common as dxgi;

impl From<ShaderFormat> for dxgi::DXGI_FORMAT {
    fn from(format: ShaderFormat) -> Self {
        match format {
            ShaderFormat::Unknown => dxgi::DXGI_FORMAT_UNKNOWN,
            ShaderFormat::R8Unorm => dxgi::DXGI_FORMAT_R8_UNORM,
            ShaderFormat::R8Uint => dxgi::DXGI_FORMAT_R8_UINT,
            ShaderFormat::R8Sint => dxgi::DXGI_FORMAT_R8_SINT,
            ShaderFormat::R8G8Unorm => dxgi::DXGI_FORMAT_R8G8_UNORM,
            ShaderFormat::R8G8Uint => dxgi::DXGI_FORMAT_R8G8_UINT,
            ShaderFormat::R8G8Sint => dxgi::DXGI_FORMAT_R8G8_SINT,
            ShaderFormat::R8G8B8A8Unorm => dxgi::DXGI_FORMAT_R8G8B8A8_UNORM,
            ShaderFormat::R8G8B8A8Uint => dxgi::DXGI_FORMAT_R8G8B8A8_UINT,
            ShaderFormat::R8G8B8A8Sint => dxgi::DXGI_FORMAT_R8G8B8A8_SINT,
            ShaderFormat::R8G8B8A8Srgb => dxgi::DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
            ShaderFormat::A2B10G10R10UnormPack32 => dxgi::DXGI_FORMAT_R10G10B10A2_UNORM,
            ShaderFormat::A2B10G10R10UintPack32 => dxgi::DXGI_FORMAT_R10G10B10A2_UINT,
            ShaderFormat::R16Uint => dxgi::DXGI_FORMAT_R16_UINT,
            ShaderFormat::R16Sint => dxgi::DXGI_FORMAT_R16_SINT,
            ShaderFormat::R16Sfloat => dxgi::DXGI_FORMAT_R16_FLOAT,
            ShaderFormat::R16G16Uint => dxgi::DXGI_FORMAT_R16G16_UINT,
            ShaderFormat::R16G16Sint => dxgi::DXGI_FORMAT_R16G16_SINT,
            ShaderFormat::R16G16Sfloat => dxgi::DXGI_FORMAT_R16G16_FLOAT,
            ShaderFormat::R16G16B16A16Uint => dxgi::DXGI_FORMAT_R16G16B16A16_UINT,
            ShaderFormat::R16G16B16A16Sint => dxgi::DXGI_FORMAT_R16G16B16A16_SINT,
            ShaderFormat::R16G16B16A16Sfloat => dxgi::DXGI_FORMAT_R16G16B16A16_FLOAT,
            ShaderFormat::R32Uint => dxgi::DXGI_FORMAT_R32_UINT,
            ShaderFormat::R32Sint =>dxgi::DXGI_FORMAT_R32_SINT,
            ShaderFormat::R32Sfloat => dxgi::DXGI_FORMAT_R32_FLOAT,
            ShaderFormat::R32G32Uint => dxgi::DXGI_FORMAT_R32G32_UINT,
            ShaderFormat::R32G32Sint => dxgi::DXGI_FORMAT_R32G32_SINT,
            ShaderFormat::R32G32Sfloat => dxgi::DXGI_FORMAT_R32G32_FLOAT,
            ShaderFormat::R32G32B32A32Uint => dxgi::DXGI_FORMAT_R32G32B32A32_UINT,
            ShaderFormat::R32G32B32A32Sint => dxgi::DXGI_FORMAT_R32G32B32A32_SINT,
            ShaderFormat::R32G32B32A32Sfloat => dxgi::DXGI_FORMAT_R32G32B32A32_FLOAT,
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
