use std::error::Error;

/// Common trait for filter chains.
pub trait FilterChain {
    /// The error type for a filter chain.
    type Error: Error;

    /// The type for the input surface to apply a filter chain to.
    type Input<'a>;

    /// The type for the output surface, including viewport information, to output the result of a
    /// filter chain to.
    type Viewport<'a>;

    /// The per-frame options to pass to the filter chain.
    type FrameOptions;

    /// Process a frame with the input image.
    ///
    /// The output image should be written to the viewport. It is expected that the viewport
    /// contains some sort of handle with interior mutability to a GPU buffer, e.g. a
    /// `RenderTargetView`, `vkImage`, or a texture `GLuint`.
    fn frame<'a>(
        &mut self,
        input: Self::Input<'a>,
        viewport: &Self::Viewport<'a>,
        frame_count: usize,
        options: Option<&Self::FrameOptions>,
    ) -> Result<(), Self::Error>;
}