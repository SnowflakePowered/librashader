use crate::{FilterMode, ShaderFormat, WrapMode};

impl From<ShaderFormat> for gl::types::GLenum {
    fn from(format: ShaderFormat) -> Self {
        match format {
            ShaderFormat::Unknown => 0 as gl::types::GLenum,
            ShaderFormat::R8Unorm => gl::R8,
            ShaderFormat::R8Uint => gl::R8UI,
            ShaderFormat::R8Sint => gl::R8I,
            ShaderFormat::R8G8Unorm => gl::RG8,
            ShaderFormat::R8G8Uint => gl::RG8UI,
            ShaderFormat::R8G8Sint => gl::RG8I,
            ShaderFormat::R8G8B8A8Unorm => gl::RGBA8,
            ShaderFormat::R8G8B8A8Uint => gl::RGBA8UI,
            ShaderFormat::R8G8B8A8Sint => gl::RGBA8I,
            ShaderFormat::R8G8B8A8Srgb => gl::SRGB8_ALPHA8,
            ShaderFormat::A2B10G10R10UnormPack32 => gl::RGB10_A2,
            ShaderFormat::A2B10G10R10UintPack32 => gl::RGB10_A2UI,
            ShaderFormat::R16Uint => gl::R16UI,
            ShaderFormat::R16Sint => gl::R16I,
            ShaderFormat::R16Sfloat => gl::R16F,
            ShaderFormat::R16G16Uint => gl::RG16UI,
            ShaderFormat::R16G16Sint => gl::RG16I,
            ShaderFormat::R16G16Sfloat => gl::RG16F,
            ShaderFormat::R16G16B16A16Uint => gl::RGBA16UI,
            ShaderFormat::R16G16B16A16Sint => gl::RGBA16I,
            ShaderFormat::R16G16B16A16Sfloat => gl::RGBA16F,
            ShaderFormat::R32Uint => gl::R32UI,
            ShaderFormat::R32Sint => gl::R32I,
            ShaderFormat::R32Sfloat => gl::R32F,
            ShaderFormat::R32G32Uint => gl::RG32UI,
            ShaderFormat::R32G32Sint => gl::RG32I,
            ShaderFormat::R32G32Sfloat => gl::RG32F,
            ShaderFormat::R32G32B32A32Uint => gl::RGBA32UI,
            ShaderFormat::R32G32B32A32Sint => gl::RGBA32I,
            ShaderFormat::R32G32B32A32Sfloat => gl::RGBA32F
        }
    }
}

impl From<WrapMode> for gl::types::GLenum {
    fn from(value: WrapMode) -> Self {
        match value {
            WrapMode::ClampToBorder => gl::CLAMP_TO_BORDER,
            WrapMode::ClampToEdge => gl::CLAMP_TO_EDGE,
            WrapMode::Repeat => gl::REPEAT,
            WrapMode::MirroredRepeat => gl::MIRRORED_REPEAT
        }
    }
}
