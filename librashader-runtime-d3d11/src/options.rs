#[repr(C)]
#[derive(Debug, Clone)]
pub struct FrameOptions {
    pub clear_history: bool,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilterChainOptions {
    pub use_deferred_context: bool,
}
