/// Options for each Vulkan shader frame.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptions {
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of the frame. 1 should be vertical.
    pub frame_direction: i32,
}

/// Options for filter chain creation.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilterChainOptions {
    /// Whether or not to explicitly disable mipmap generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
}

