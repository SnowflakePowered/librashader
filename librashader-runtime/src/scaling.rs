use crate::filter_pass::FilterPassMeta;
use crate::framebuffer::FramebufferPool;
use crate::scaling;
use librashader_common::{ImageFormat, Size};
use librashader_presets::{Scale2D, ScaleFactor, ScaleType, Scaling};
use num_traits::AsPrimitive;
use std::ops::Mul;

pub const MAX_TEXEL_SIZE: f32 = 16384f32;

/// Trait for size scaling relative to the viewport.
pub trait ViewportSize<T>
where
    T: Mul<ScaleFactor, Output = f32> + Copy + Ord + 'static,
    f32: AsPrimitive<T>,
{
    /// Produce a `Size<T>` scaled with the input scaling options.
    /// The size will at minimum be 1x1, and at a maximum of the specified clamp
    /// value, or 16384x16384 if the clamp value is not specified.
    fn scale_viewport(
        self,
        scaling: Scale2D,
        viewport: Size<T>,
        original: Size<T>,
        clamp: Option<T>,
    ) -> Size<T>;
}

impl<T> ViewportSize<T> for Size<T>
where
    T: Mul<ScaleFactor, Output = f32> + Copy + Ord + 'static,
    f32: AsPrimitive<T>,
{
    fn scale_viewport(
        self,
        scaling: Scale2D,
        viewport: Size<T>,
        original: Size<T>,
        clamp: Option<T>,
    ) -> Size<T>
    where
        T: Mul<ScaleFactor, Output = f32> + Copy + Ord + 'static,
        f32: AsPrimitive<T>,
    {
        scaling::scale(scaling, self, viewport, original, clamp)
    }
}

/// Trait for size scaling relating to mipmap generation.
pub trait MipmapSize<T> {
    /// Calculate the number of mipmap levels for a given size.
    fn calculate_miplevels(self) -> T;

    /// Scale the size according to the given mipmap level.
    /// The size will at minimum be 1x1, and at a maximum 16384x16384.
    fn scale_mipmap(self, miplevel: T) -> Size<T>;
}

impl MipmapSize<u32> for Size<u32> {
    fn calculate_miplevels(self) -> u32 {
        let mut size = std::cmp::max(self.width, self.height);
        let mut levels = 0;
        while size != 0 {
            levels += 1;
            size >>= 1;
        }

        levels
    }

    fn scale_mipmap(self, miplevel: u32) -> Size<u32> {
        let scaled_width = std::cmp::max(self.width >> miplevel, 1);
        let scaled_height = std::cmp::max(self.height >> miplevel, 1);
        Size::new(scaled_width, scaled_height)
    }
}

fn scale<T>(
    scaling: Scale2D,
    source: Size<T>,
    viewport: Size<T>,
    original: Size<T>,
    clamp: Option<T>,
) -> Size<T>
where
    T: Mul<ScaleFactor, Output = f32> + Copy + Ord + 'static,
    f32: AsPrimitive<T>,
{
    let width = match scaling.x {
        Scaling {
            scale_type: ScaleType::Input,
            factor,
        } => source.width * factor,
        Scaling {
            scale_type: ScaleType::Absolute,
            factor,
        } => factor.into(),
        Scaling {
            scale_type: ScaleType::Viewport,
            factor,
        } => viewport.width * factor,
        Scaling {
            scale_type: ScaleType::Original,
            factor,
        } => original.width * factor,
    };

    let height = match scaling.y {
        Scaling {
            scale_type: ScaleType::Input,
            factor,
        } => source.height * factor,
        Scaling {
            scale_type: ScaleType::Absolute,
            factor,
        } => factor.into(),
        Scaling {
            scale_type: ScaleType::Viewport,
            factor,
        } => viewport.height * factor,
        Scaling {
            scale_type: ScaleType::Original,
            factor,
        } => original.height * factor,
    };

    Size {
        width: std::cmp::min(
            std::cmp::max(width.round().as_(), 1f32.as_()),
            clamp.unwrap_or(MAX_TEXEL_SIZE.as_()),
        ),
        height: std::cmp::min(
            std::cmp::max(height.round().as_(), 1f32.as_()),
            clamp.unwrap_or(MAX_TEXEL_SIZE.as_()),
        ),
    }
}

/// Trait for owned framebuffer objects that can be scaled.
pub trait ScaleFramebuffer<T = ()> {
    type Error;
    type Context;
    /// Scale the framebuffer according to the provided parameters, returning the new size.
    fn scale(
        &mut self,
        scaling: Scale2D,
        format: ImageFormat,
        viewport_size: &Size<u32>,
        source_size: &Size<u32>,
        original_size: &Size<u32>,
        should_mipmap: bool,
        context: &Self::Context,
    ) -> Result<Size<u32>, Self::Error>;

    /// Scale the sparse feedback framebuffers, invoking `callback` for each pass that is
    /// referenced as feedback so the runtime can refresh its bound feedback texture.
    #[inline(always)]
    fn scale_feedback_framebuffers<P>(
        source_size: Size<u32>,
        viewport_size: Size<u32>,
        original_size: Size<u32>,
        feedback: &mut FramebufferPool<Self>,
        passes: &[P],
        callback: impl FnMut(usize, &P, &Self) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
        Self::Context: Default,
        P: FilterPassMeta,
    {
        Self::scale_feedback_framebuffers_with_context(
            source_size,
            viewport_size,
            original_size,
            feedback,
            passes,
            &Self::Context::default(),
            callback,
        )
    }

    /// Scale the sparse feedback framebuffers with a user provided context.
    #[inline(always)]
    fn scale_feedback_framebuffers_with_context<P>(
        source_size: Size<u32>,
        viewport_size: Size<u32>,
        original_size: Size<u32>,
        feedback: &mut FramebufferPool<Self>,
        passes: &[P],
        context: &Self::Context,
        callback: impl FnMut(usize, &P, &Self) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
        P: FilterPassMeta,
    {
        scale_feedback_framebuffers_callback::<T, Self, Self::Error, Self::Context, P, _>(
            source_size,
            viewport_size,
            original_size,
            feedback,
            passes,
            context,
            callback,
        )
    }

    /// Scale the pooled output framebuffers, invoking `callback` for each pass with its
    /// routed render target and scaled output size so the runtime can draw it.
    #[inline(always)]
    fn scale_output_framebuffers<P>(
        source_size: Size<u32>,
        viewport_size: Size<u32>,
        original_size: Size<u32>,
        output: &mut FramebufferPool<Self>,
        passes: &mut [P],
        callback: impl FnMut(usize, &mut P, &Self, Size<u32>) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
        Self::Context: Default,
        P: FilterPassMeta,
    {
        Self::scale_output_framebuffers_with_context(
            source_size,
            viewport_size,
            original_size,
            output,
            passes,
            &Self::Context::default(),
            callback,
        )
    }

    /// Scale the pooled output framebuffers with a user provided context.
    #[inline(always)]
    fn scale_output_framebuffers_with_context<P>(
        source_size: Size<u32>,
        viewport_size: Size<u32>,
        original_size: Size<u32>,
        output: &mut FramebufferPool<Self>,
        passes: &mut [P],
        context: &Self::Context,
        callback: impl FnMut(usize, &mut P, &Self, Size<u32>) -> Result<(), Self::Error>,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
        P: FilterPassMeta,
    {
        scale_output_framebuffers_callback::<T, Self, Self::Error, Self::Context, P, _>(
            source_size,
            viewport_size,
            original_size,
            output,
            passes,
            context,
            callback,
        )
    }
}

#[inline(always)]
fn scale_feedback_framebuffers_callback<T, F, E, C, P, CB>(
    source_size: Size<u32>,
    viewport_size: Size<u32>,
    original_size: Size<u32>,
    feedback: &mut FramebufferPool<F>,
    passes: &[P],
    context: &C,
    mut callback: CB,
) -> Result<(), E>
where
    F: ScaleFramebuffer<T, Context = C, Error = E>,
    P: FilterPassMeta,
    CB: FnMut(usize, &P, &F) -> Result<(), E>,
{
    let mut iterator = passes.iter().enumerate().peekable();
    let mut target_size = source_size;
    while let Some((index, pass)) = iterator.next() {
        let should_mipmap = iterator
            .peek()
            .map_or(false, |(_, p)| p.meta().mipmap_input);

        let next_size = target_size.scale_viewport(
            pass.meta().scaling.clone(),
            viewport_size,
            original_size,
            None,
        );

        if feedback.contains(index) {
            feedback[index].scale(
                pass.meta().scaling.clone(),
                pass.get_format(),
                &viewport_size,
                &target_size,
                &original_size,
                should_mipmap,
                context,
            )?;
            callback(index, pass, &feedback[index])?;
        }

        target_size = next_size;
    }

    Ok(())
}

#[inline(always)]
fn scale_output_framebuffers_callback<T, F, E, C, P, CB>(
    source_size: Size<u32>,
    viewport_size: Size<u32>,
    original_size: Size<u32>,
    output: &mut FramebufferPool<F>,
    passes: &mut [P],
    context: &C,
    mut callback: CB,
) -> Result<(), E>
where
    F: ScaleFramebuffer<T, Context = C, Error = E>,
    P: FilterPassMeta,
    CB: FnMut(usize, &mut P, &F, Size<u32>) -> Result<(), E>,
{
    let len = passes.len();

    // Compute every pass's output size up front so the pool can be colored by liveness
    // before any buffer is touched.
    let mut sizes = Vec::with_capacity(len);
    let mut target_size = source_size;
    for pass in passes.iter() {
        target_size = target_size.scale_viewport(
            pass.meta().scaling.clone(),
            viewport_size,
            original_size,
            None,
        );
        sizes.push(target_size);
    }

    output.prepare(&sizes);

    for index in 0..len {
        let scaling = passes[index].meta().scaling.clone();
        let format = passes[index].get_format();
        let should_mipmap = passes
            .get(index + 1)
            .map_or(false, |p| p.meta().mipmap_input);
        let prev = if index == 0 {
            source_size
        } else {
            sizes[index - 1]
        };

        let size = output[index].scale(
            scaling,
            format,
            &viewport_size,
            &prev,
            &original_size,
            should_mipmap,
            context,
        )?;

        callback(index, &mut passes[index], &output[index], size)?;
    }

    Ok(())
}
