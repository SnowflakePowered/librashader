//! Direct3D 12 shader runtime errors.
//!

use d3d12_descriptor_heap::D3D12DescriptorHeapError;
use thiserror::Error;
use windows::Win32::Graphics::Direct3D12::D3D12_RESOURCE_DIMENSION;

/// Cumulative error type for Direct3D12 filter chains.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum FilterChainError {
    #[error("invariant assumption about d3d12 did not hold. report this as an issue.")]
    Direct3DOperationError(&'static str),
    #[error("direct3d driver error: {0}")]
    Direct3DError(#[from] windows::core::Error),
    #[error("shader preset parse error: {0}")]
    ShaderPresetError(#[from] ParsePresetError),
    #[error("shader preprocess error: {0}")]
    ShaderPreprocessError(#[from] PreprocessError),
    #[error("shader compile error: {0}")]
    ShaderCompileError(#[from] ShaderCompileError),
    #[error("shader reflect error: {0}")]
    ShaderReflectError(#[from] ShaderReflectError),
    #[error("lut loading error: {0}")]
    LutLoadError(#[from] ImageError),
    #[error("heap error: {0}")]
    HeapError(#[from] D3D12DescriptorHeapError),
    #[error("allocation error: {0}")]
    AllocationError(#[from] gpu_allocator::AllocationError),
    #[error("invalid resource dimension {0:?}")]
    InvalidDimensionError(D3D12_RESOURCE_DIMENSION),
    #[error("unreachable")]
    Infallible(#[from] std::convert::Infallible),
}

/// Result type for Direct3D 12 filter chains.
pub type Result<T> = std::result::Result<T, FilterChainError>;

macro_rules! assume_d3d12_init {
    ($value:ident, $call:literal) => {
        let $value = $value.ok_or($crate::error::FilterChainError::Direct3DOperationError(
            $call,
        ))?;
    };
    (mut $value:ident, $call:literal) => {
        let mut $value = $value.ok_or($crate::error::FilterChainError::Direct3DOperationError(
            $call,
        ))?;
    };
}

/// Macro for unwrapping result of a D3D function.
pub(crate) use assume_d3d12_init;
use librashader_preprocess::PreprocessError;
use librashader_presets::ParsePresetError;
use librashader_reflect::error::{ShaderCompileError, ShaderReflectError};
use librashader_runtime::image::ImageError;
