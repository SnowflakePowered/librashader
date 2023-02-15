//! Direct3D 12 shader runtime options.

/// Options for each Direct3D12 shader frame.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct FrameOptionsD3D12 {
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of rendering.
    /// -1 indicates that the frames are played in reverse order.
    pub frame_direction: i32,
}

/// Options for Direct3D 12 filter chain creation.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct FilterChainOptionsD3D12 {
    /// Force the HLSL shader pipeline. This may reduce shader compatibility.
    pub force_hlsl_pipeline: bool,

    /// Whether or not to explicitly disable mipmap
    /// generation for intermediate passes regardless
    /// of shader preset settings.
    pub force_no_mipmaps: bool,

    /// Disable the shader object cache. Shaders will be
    /// recompiled rather than loaded from the cache.
    pub disable_cache: bool,
}
