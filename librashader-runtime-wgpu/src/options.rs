//! wgpu shader runtime options.

use librashader_common::ColorSpace;
use librashader_runtime::impl_default_frame_options;
impl_default_frame_options!(FrameOptionsWgpu);

/// Options for filter chain creation.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct FilterChainOptionsWgpu {
    /// Whether or not to explicitly disable mipmap generation regardless of shader preset settings.
    pub force_no_mipmaps: bool,
    /// Enable the shader object cache. Shaders will be loaded from the cache
    /// if this is enabled.
    pub enable_cache: bool,
    /// WGPU adapter info for use to determine the name of the pipeline cache index.
    /// If this is not provided, then it will fallback to a default "wgpu" index, which
    /// may clobber the cache for a different device using WGPU.
    pub adapter_info: Option<wgpu::AdapterInfo>,
    /// If HDR is enabled, the HDR color space of the final output pass.
    /// For non-HDR shaders, this should always be [`ColorSpace::Sdr`].
    ///
    /// Use [`ShaderPreset::color_space`](librashader_presets::PresetColorSpace::color_space)
    /// to determine if an HDR color space is required.
    pub color_space: ColorSpace,
}
