#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptionsD3D11 {
    pub clear_history: bool,
    pub frame_direction: i32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilterChainOptionsD3D11 {
    pub use_deferred_context: bool,
}
