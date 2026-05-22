//! Direct3D 9 shader runtime options.

use librashader_common::ColorSpace;
use librashader_runtime::impl_default_frame_options;
impl_default_frame_options!(FrameOptionsD3D9);

/// Options for Direct3D 9 filter chain creation.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct FilterChainOptionsD3D9 {
    /// Whether or not to explicitly disable mipmap
    /// generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
    /// Disable the shader object cache. Shaders will be
    /// recompiled rather than loaded from the cache.
    pub disable_cache: bool,
    /// If HDR is enabled, the HDR color space of the final output pass.
    /// For non-HDR shaders, this should always be [`ColorSpace::Sdr`].
    ///
    /// Use [`ShaderPreset::color_space`](librashader_presets::PresetColorSpace::color_space)
    /// to determine if an HDR color space is required.
    ///
    /// D3D9 has no HDR surface formats, so HDR shaders will not display
    /// correctly on D3D9.
    pub color_space: ColorSpace,
}
