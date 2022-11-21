use gl::types::GLenum;
use librashader_common::image;
use librashader_preprocess::PreprocessError;
use librashader_presets::ParsePresetError;
use librashader_reflect::error::{ShaderCompileError, ShaderReflectError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FilterChainError {
    #[error("fbo initialization error")]
    FramebufferInit(GLenum),
    #[error("SPIRV reflection error")]
    SpirvCrossReflectError(#[from] spirv_cross::ErrorCode),
    #[error("shader preset parse error")]
    ShaderPresetError(#[from] ParsePresetError),
    #[error("shader preprocess error")]
    ShaderPreprocessError(#[from] PreprocessError),
    #[error("shader compile error")]
    ShaderCompileError(#[from] ShaderCompileError),
    #[error("shader reflect error")]
    ShaderReflectError(#[from] ShaderReflectError),
    #[error("lut loading error")]
    LutLoadError(#[from] image::ImageError)
}

pub type Result<T> = std::result::Result<T, FilterChainError>;