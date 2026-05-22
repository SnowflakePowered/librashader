//! OpenGL shader runtime options.

use librashader_common::ColorSpace;
use librashader_runtime::impl_default_frame_options;
impl_default_frame_options!(FrameOptionsGL);

/// Options for filter chain creation.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct FilterChainOptionsGL {
    /// The GLSL version. Should be at least `330`.
    pub glsl_version: u16,
    /// Whether or not to use the Direct State Access APIs. Only available on OpenGL 4.5+.
    /// If this is off, compiled program caching will not be available.
    pub use_dsa: bool,
    /// Whether or not to explicitly disable mipmap generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
    /// Disable the shader object cache. Shaders will be recompiled rather than loaded from the cache.
    pub disable_cache: bool,
    /// If HDR is enabled, the HDR color space of the final output pass.
    /// For non-HDR shaders, this should always be [`ColorSpace::Sdr`].
    ///
    /// Use [`ShaderPreset::color_space`](librashader_presets::PresetColorSpace::color_space)
    /// to determine if an HDR color space is required.
    pub color_space: ColorSpace,
}
