#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptionsGL {
    pub clear_history: bool,
    pub frame_direction: i32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilterChainOptionsGL {
    pub gl_version: u16,
    pub use_dsa: bool,
}
