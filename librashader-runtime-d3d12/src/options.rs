//! Direct3D12 shader runtime options.

/// Options for each Direct3D11 shader frame.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptionsD3D12 {
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of the frame. 1 should be vertical.
    pub frame_direction: i32,
}

/// Options for Direct3D11 filter chain creation.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilterChainOptionsD3D12 {
    /// Force the HLSL shader pipeline. This may reduce shader compatibility
    pub force_hlsl_pipeline: bool,

    /// Whether or not to explicitly disable mipmap
    /// generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
}
