//! OpenGL shader runtime options.

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
    /// Do a full state GL save and restore before rendering. This defaults to false, but
    /// should be enabled if the host application is sensitive to GL state.
    ///
    /// Even if this setting is false, `SCISSOR_TEST`, `CULL_FACE`, `BLEND`,
    /// `DEPTH_TEST`, and `STENCIL_TEST` will be disabled during the duration of
    /// the call to `frame` and restored after `frame` returns.
    pub save_gl_state: bool,
}
