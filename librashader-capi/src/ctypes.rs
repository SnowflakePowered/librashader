//! Binding types for the librashader C API.
use crate::error::LibrashaderError;
use librashader::presets::ShaderPreset;
use std::ptr::NonNull;

pub type libra_shader_preset_t = Option<NonNull<ShaderPreset>>;
pub type libra_error_t = Option<NonNull<LibrashaderError>>;

#[cfg(feature = "runtime-opengl")]
pub type libra_gl_filter_chain_t = Option<NonNull<librashader::runtime::gl::FilterChain>>;

#[cfg(feature = "runtime-d3d11")]
pub type libra_d3d11_filter_chain_t =
    Option<NonNull<librashader::runtime::d3d11::FilterChain>>;

/// Parameters for the output viewport.
#[repr(C)]
pub struct libra_viewport_t {
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
}
