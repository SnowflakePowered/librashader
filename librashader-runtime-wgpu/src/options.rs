/// Options for each WGPU shader frame.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct FrameOptionsWGPU {
    /// Whether or not to clear the history buffers.
    pub clear_history: bool,
    /// The direction of rendering.
    /// -1 indicates that the frames are played in reverse order.
    pub frame_direction: i32,
}
