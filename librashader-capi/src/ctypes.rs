use std::mem::ManuallyDrop;
use librashader::presets::ShaderPreset;
use crate::error::LibrashaderError;

pub type libra_shader_preset_t = ManuallyDrop<Option<Box<ShaderPreset>>>;
pub type libra_error_t = *const LibrashaderError;

// #[cfg(feature = "runtime-opengl")]
pub type libra_gl_filter_chain_t = ManuallyDrop<Option<Box<librashader::runtime::gl::FilterChainGL>>>;

#[repr(C)]
pub struct libra_viewport_t {
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
}