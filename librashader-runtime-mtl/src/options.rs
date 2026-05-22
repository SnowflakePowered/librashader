//! Metal shader runtime options.

use librashader_common::ColorSpace;
use librashader_runtime::impl_default_frame_options;
impl_default_frame_options!(FrameOptionsMetal);

/// Options for filter chain creation.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct FilterChainOptionsMetal {
    /// Whether or not to explicitly disable mipmap generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
    /// If HDR is enabled, the HDR color space of the final output pass.
    /// For non-HDR shaders, this should always be [`ColorSpace::Sdr`].
    ///
    /// Use [`ShaderPreset::color_space`](librashader_presets::PresetColorSpace::color_space)
    /// to determine if an HDR color space is required.
    pub color_space: ColorSpace,
}
