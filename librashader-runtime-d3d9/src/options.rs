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
    /// HDR output mode bound to the shader `HDRMode` uniform. D3D9 has no
    /// HDR surface formats, so this is plumbed to the shader uniform but
    /// `output_color_space()` always reports `Srgb` for D3D9 chains.
    pub hdr_mode: ColorSpace,
}
