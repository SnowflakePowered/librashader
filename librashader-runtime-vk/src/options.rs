//! Vulkan shader runtime options.

/// Options for each Vulkan shader frame.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptionsVulkan {
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of the frame. 1 should be vertical.
    pub frame_direction: i32,
}

/// Options for filter chain creation.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilterChainOptionsVulkan {
    /// The number of frames in flight to keep. If zero, defaults to three.
    pub frames_in_flight: u32,
    /// Whether or not to explicitly disable mipmap generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
}

