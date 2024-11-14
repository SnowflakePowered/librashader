use bitflags::bitflags;

bitflags! {
    /// Enable feature flags for shaders.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(transparent))]
    pub struct ShaderFeatures: u32 {
        /// No features are enabled.
        const NONE = 0b00000000;
        /// Enable `OriginalAspect` and `OriginalAspectRotated` uniforms.
        ///
        /// Note that this flag only enables the `_HAS_ORIGINALASPECT_UNIFORMS` define.
        /// The uniforms will be bound unconditionally if found in reflection.
        const ORIGINAL_ASPECT_UNIFORMS = 0b00000001;
        /// Enable `FrameTimeDelta` and `OriginalFPS` uniforms.
        ///
        /// Note that this flag only enables the `_HAS_FRAMETIME_UNIFORMS` define.
        /// The uniforms will be bound unconditionally if found in reflection.
        const FRAMETIME_UNIFORMS = 0b00000010;
    }
}
