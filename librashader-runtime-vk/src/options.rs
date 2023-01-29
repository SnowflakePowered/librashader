//! Vulkan shader runtime options.

use ash::vk;

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
    /// The format to use for the render pass. If this is `VK_FORMAT_UNDEFINED`, dynamic rendering
    /// will be used instead of a render pass. If this is set to some format, the render passes
    /// will be created with such format. It is recommended if possible to use dynamic rendering,
    /// because render-pass mode will create new framebuffers per pass.
    pub render_pass_format: vk::Format,
}
