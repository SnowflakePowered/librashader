use crate::Size;

/// The rendering output of a filter chain.
///
/// Viewport coordinates are relative to the coordinate system of the
/// target runtime. For correct results, `x` and `y`  should almost always be
/// 0, and `size` should be the same as the size of the output texture.
///
/// Size uniforms will always be passed the full size of the output texture,
/// regardless of the user-specified viewport size.
pub struct Viewport<'a, T> {
    /// The x offset to start rendering from. For correct results, this should almost
    /// always be 0 to indicate the origin.
    pub x: f32,
    /// The y offset to begin rendering from.
    pub y: f32,
    /// An optional pointer to an MVP to use when rendering
    /// to the viewport.
    pub mvp: Option<&'a [f32; 16]>,
    /// The output handle to render the final image to.
    pub output: T,
    /// The extent of the viewport size starting from the origin defined
    /// by x and y.
    pub size: Size<u32>,
}
