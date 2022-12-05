/// Options for each Direct3D11 shader frame.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptionsD3D11 {
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of the frame. 1 should be vertical.
    pub frame_direction: i32,
}

/// Options for Direct3D11 filter chain creation.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilterChainOptionsD3D11 {
    /// Use a deferred context to record shader rendering state.
    ///
    /// The deferred context will be executed on the immediate context
    /// with `RenderContextState = true`.
    pub use_deferred_context: bool,
}
