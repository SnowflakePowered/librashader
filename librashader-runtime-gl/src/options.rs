#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptionsGL {
    pub clear_history: bool,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilterChainOptionsGL {
    pub gl_version: u16,
    pub use_dsa: bool,
}
