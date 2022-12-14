/// Options for each OpenGL shader frame.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptionsGL {
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of the frame. 1 should be vertical.
    pub frame_direction: i32,
}

/// Options for filter chain creation.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilterChainOptionsGL {
    /// The GLSL version. Should be at least `330`.
    pub gl_version: u16,
    /// Whether or not to use the Direct State Access APIs. Only available on OpenGL 4.5+.
    pub use_dsa: bool,
}
