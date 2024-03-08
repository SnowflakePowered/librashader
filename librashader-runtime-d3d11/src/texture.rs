use crate::error::Result;
use crate::framebuffer::OwnedImage;
use librashader_common::{FilterMode, WrapMode};
use windows::Win32::Graphics::Direct3D11::{ID3D11RenderTargetView, ID3D11ShaderResourceView};

/// An image view for use as a shader resource.
///
/// Contains an `ID3D11ShaderResourceView`, and a size.
#[derive(Debug, Clone)]
pub struct D3D11InputView {
    /// A handle to the shader resource view.
    pub handle: ID3D11ShaderResourceView,
}

/// An image view for use as a render target.
///
/// Contains an `ID3D11RenderTargetView`, and a size.
#[derive(Debug, Clone)]
pub struct D3D11OutputView {
    /// A handle to the render target view.
    pub handle: ID3D11RenderTargetView,
}

#[derive(Debug, Clone)]
pub struct InputTexture {
    pub view: ID3D11ShaderResourceView,
    pub filter: FilterMode,
    pub wrap_mode: WrapMode,
}

impl InputTexture {
    pub(crate) fn from_framebuffer(
        fbo: &OwnedImage,
        wrap_mode: WrapMode,
        filter: FilterMode,
    ) -> Result<Self> {
        Ok(InputTexture {
            view: fbo.create_shader_resource_view()?,
            filter,
            wrap_mode,
        })
    }
}

impl AsRef<InputTexture> for InputTexture {
    fn as_ref(&self) -> &InputTexture {
        self
    }
}
