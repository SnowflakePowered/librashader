//! Binding types for the librashader C API.
use crate::error::LibrashaderError;
use librashader::presets::ShaderPreset;
use std::ptr::NonNull;

/// A handle to a shader preset object.
pub type libra_shader_preset_t = Option<NonNull<ShaderPreset>>;

/// A handle to a librashader error object.
pub type libra_error_t = Option<NonNull<LibrashaderError>>;

/// A handle to a OpenGL filter chain.
#[cfg(feature = "runtime-opengl")]
#[doc(cfg(feature = "runtime-opengl"))]
pub type libra_gl_filter_chain_t = Option<NonNull<librashader::runtime::gl::capi::FilterChainGL>>;

/// A handle to a Direct3D11 filter chain.
#[cfg(all(target_os = "windows", feature = "runtime-d3d11"))]
#[doc(cfg(all(target_os = "windows", feature = "runtime-d3d11")))]
pub type libra_d3d11_filter_chain_t =
    Option<NonNull<librashader::runtime::d3d11::capi::FilterChainD3D11>>;

/// A handle to a Vulkan filter chain.
#[cfg(feature = "runtime-vulkan")]
#[doc(cfg(feature = "runtime-vulkan"))]
pub type libra_vk_filter_chain_t =
    Option<NonNull<librashader::runtime::vk::capi::FilterChainVulkan>>;

/// Defines the output viewport for a rendered frame.
#[repr(C)]
pub struct libra_viewport_t {
    /// The x offset in the viewport framebuffer to begin rendering from.
    pub x: f32,
    /// The y offset in the viewport framebuffer to begin rendering from.
    pub y: f32,
    /// The width of the viewport framebuffer.
    pub width: u32,
    /// The height of the viewport framebuffer.
    pub height: u32,
}
