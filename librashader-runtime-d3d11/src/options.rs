//! Direct3D 11 shader runtime options.

/// Options for each Direct3D 11 shader frame.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct FrameOptionsD3D11 {
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of rendering.
    /// -1 indicates that the frames are played in reverse order.
    pub frame_direction: i32,
}

/// Options for Direct3D 11 filter chain creation.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct FilterChainOptionsD3D11 {
    /// Use a deferred context to record shader rendering state.
    ///
    /// The deferred context will be executed on the immediate context
    /// with `RenderContextState = true`.
    pub use_deferred_context: bool,

    /// Whether or not to explicitly disable mipmap
    /// generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
}
