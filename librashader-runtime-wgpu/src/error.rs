//! Vulkan shader runtime errors.
use librashader_preprocess::PreprocessError;
use librashader_presets::ParsePresetError;
use librashader_reflect::error::{ShaderCompileError, ShaderReflectError};
use librashader_runtime::image::ImageError;
use std::convert::Infallible;
use thiserror::Error;

/// Cumulative error type for WGPU filter chains.
#[derive(Error, Debug)]
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
}

impl From<Infallible> for FilterChainError {
    fn from(_value: Infallible) -> Self {
        panic!("uninhabited error")
    }
}

/// Result type for Vulkan filter chains.
pub type Result<T> = std::result::Result<T, FilterChainError>;
