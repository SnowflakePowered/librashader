#[cfg(feature = "opengl")]
pub mod gl;
#[cfg(feature = "dxgi")]
pub mod dx;

pub mod image;
pub mod runtime;


use std::convert::Infallible;
use std::str::FromStr;
use num_traits::AsPrimitive;

#[repr(u32)]
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq)]
pub enum ShaderFormat {
    #[default]
    Unknown = 0,

    /* 8-bit */
    R8Unorm,
    R8Uint,
    R8Sint,
    R8G8Unorm,
    R8G8Uint,
    R8G8Sint,
    R8G8B8A8Unorm,
    R8G8B8A8Uint,
    R8G8B8A8Sint,
    R8G8B8A8Srgb,

    /* 10-bit */
    A2B10G10R10UnormPack32,
    A2B10G10R10UintPack32,

    /* 16-bit */
    R16Uint,
    R16Sint,
    R16Sfloat,
    R16G16Uint,
    R16G16Sint,
    R16G16Sfloat,
    R16G16B16A16Uint,
    R16G16B16A16Sint,
    R16G16B16A16Sfloat,

    /* 32-bit */
    R32Uint,
    R32Sint,
    R32Sfloat,
    R32G32Uint,
    R32G32Sint,
    R32G32Sfloat,
    R32G32B32A32Uint,
    R32G32B32A32Sint,
    R32G32B32A32Sfloat,
}

#[repr(i32)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub enum FilterMode {
    #[default]
    Linear = 0,
    Nearest,
    Unspecified,
}

impl FromStr for WrapMode {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "clamp_to_border" => WrapMode::ClampToBorder,
            "clamp_to_edge" => WrapMode::ClampToEdge,
            "repeat" => WrapMode::Repeat,
            "mirrored_repeat" => WrapMode::MirroredRepeat,
            _ => WrapMode::ClampToBorder,
        })
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub enum WrapMode {
    #[default]
    ClampToBorder = 0,
    ClampToEdge,
    Repeat,
    MirroredRepeat,
}

impl FromStr for ShaderFormat {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "UNKNOWN" => Self::Unknown,

            "R8_UNORM" => Self::R8Unorm,
            "R8_UINT" => Self::R8Uint,
            "R8_SINT" => Self::R8Sint,
            "R8G8_UNORM" => Self::R8G8Unorm,
            "R8G8_UINT" => Self::R8Uint,
            "R8G8_SINT" => Self::R8G8Sint,
            "R8G8B8A8_UNORM" => Self::R8G8B8A8Unorm,
            "R8G8B8A8_UINT" => Self::R8G8B8A8Uint,
            "R8G8B8A8_SINT" => Self::R8G8B8A8Sint,
            "R8G8B8A8_SRGB" => Self::R8G8B8A8Srgb,

            "A2B10G10R10_UNORM_PACK32" => Self::A2B10G10R10UnormPack32,
            "A2B10G10R10_UINT_PACK32" => Self::A2B10G10R10UintPack32,

            "R16_UINT" => Self::R16Uint,
            "R16_SINT" => Self::R16Sint,
            "R16_SFLOAT" => Self::R16Sfloat,
            "R16G16_UINT" => Self::R16G16Uint,
            "R16G16_SINT" => Self::R16G16Sint,
            "R16G16_SFLOAT" => Self::R16G16Sfloat,
            "R16G16B16A16_UINT" => Self::R16G16B16A16Uint,
            "R16G16B16A16_SINT" => Self::R16G16B16A16Sint,
            "R16G16B16A16_SFLOAT" => Self::R16G16B16A16Sfloat,

            "R32_UINT" => Self::R32Uint,
            "R32_SINT" => Self::R32Sint,
            "R32_SFLOAT" => Self::R32Sfloat,
            "R32G32_UINT" => Self::R32G32Uint,
            "R32G32_SINT" => Self::R32G32Sint,
            "R32G32_SFLOAT" => Self::R32G32Sfloat,
            "R32G32B32A32_UINT" => Self::R32G32B32A32Uint,
            "R32G32B32A32_SINT" => Self::R32G32B32A32Sint,
            "R32G32B32A32_SFLOAT" => Self::R32G32B32A32Sfloat,
            _ => Self::Unknown,
        })
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    pub fn new(width: T, height: T) -> Self {
        Size { width, height }
    }
}

impl<T> From<Size<T>> for [f32; 4]
where T: Copy + AsPrimitive<f32>
{
    fn from(value: Size<T>) -> Self {
        [
            value.width.as_(),
            value.height.as_(),
            1.0 / value.width.as_(),
            1.0 / value.height.as_(),
        ]
    }
}
