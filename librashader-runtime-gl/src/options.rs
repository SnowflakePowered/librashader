//! OpenGL shader runtime options.

/// Options for each OpenGL shader frame.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptionsGL {
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of rendering.
    /// -1 indicates that the frames are played in reverse order.
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
    /// Whether or not to explicitly disable mipmap generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
}
