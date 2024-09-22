//! OpenGL shader runtime errors.

use librashader_preprocess::PreprocessError;
use librashader_presets::ParsePresetError;
use librashader_reflect::error::{ShaderCompileError, ShaderReflectError};
use librashader_runtime::image::ImageError;
use thiserror::Error;

/// Cumulative error type for OpenGL filter chains.
#[derive(Error, Debug)]
pub enum FilterChainError {
    #[error("fbo initialization error")]
    FramebufferInit(u32),
    #[error("SPIRV reflection error")]
    SpirvCrossReflectError(#[from] spirv_cross2::SpirvCrossError),
    #[error("shader preset parse error")]
    ShaderPresetError(#[from] ParsePresetError),
    #[error("shader preprocess error")]
    ShaderPreprocessError(#[from] PreprocessError),
    #[error("shader compile error")]
    ShaderCompileError(#[from] ShaderCompileError),
    #[error("shader reflect error")]
    ShaderReflectError(#[from] ShaderReflectError),
    #[error("lut loading error")]
    LutLoadError(#[from] ImageError),
    #[error("opengl was not initialized")]
    GLLoadError,
    #[error("opengl could not link program")]
    GLLinkError,
    #[error("opengl could not compile program")]
    GlCompileError,
    #[error("opengl could not create samplers")]
    GlSamplerError,
    #[error("opengl could not create samplers")]
    GlProgramError,
    #[error("opengl error: {0}")]
    GlError(String),
}

/// Result type for OpenGL filter chains.
pub type Result<T> = std::result::Result<T, FilterChainError>;
