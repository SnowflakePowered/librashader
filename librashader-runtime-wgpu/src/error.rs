//! wgpu shader runtime errors.
use librashader_preprocess::PreprocessError;
use librashader_presets::ParsePresetError;
use librashader_reflect::error::{ShaderCompileError, ShaderReflectError};
use librashader_runtime::image::ImageError;
use thiserror::Error;

/// Cumulative error type for wgpu filter chains.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum FilterChainError {
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
    #[error("poll error: {0}")]
    PollError(#[from] wgpu::PollError),
    #[error("unreachable")]
    Infallible(#[from] std::convert::Infallible),
}

/// Result type for wgpu filter chains.
pub type Result<T> = std::result::Result<T, FilterChainError>;
