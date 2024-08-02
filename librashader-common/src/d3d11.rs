use crate::{FilterMode, GetSize, Size, WrapMode};
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D11;

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
            FilterMode::Nearest => Direct3D11::D3D11_FILTER_MIN_MAG_MIP_POINT,
        }
    }
}

impl GetSize<u32> for Direct3D11::ID3D11RenderTargetView {
    type Error = windows::core::Error;

    fn size(&self) -> Result<Size<u32>, Self::Error> {
        let parent = unsafe { self.GetResource()?.cast::<Direct3D11::ID3D11Texture2D>()? };

        let mut desc = Default::default();
        unsafe {
            parent.GetDesc(&mut desc);
        }

        Ok(Size {
            height: desc.Height,
            width: desc.Width,
        })
    }
}

impl GetSize<u32> for Direct3D11::ID3D11ShaderResourceView {
    type Error = windows::core::Error;

    fn size(&self) -> Result<Size<u32>, Self::Error> {
        let parent = unsafe { self.GetResource()?.cast::<Direct3D11::ID3D11Texture2D>()? };

        let mut desc = Default::default();
        unsafe {
            parent.GetDesc(&mut desc);
        }

        Ok(Size {
            height: desc.Height,
            width: desc.Width,
        })
    }
}
