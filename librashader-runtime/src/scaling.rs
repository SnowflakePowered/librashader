use librashader_common::Size;
use librashader_presets::{Scale2D, ScaleFactor, ScaleType, Scaling};
use num_traits::AsPrimitive;
use std::ops::Mul;

/// Produce a `Size<T>` scaled with the input scaling options.
pub fn scale<T>(scaling: Scale2D, source: Size<T>, viewport: Size<T>) -> Size<T>
where
    T: Mul<ScaleFactor, Output = f32> + Copy + 'static,
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
    };

    Size {
        width: width.round().as_(),
        height: height.round().as_(),
    }
}

/// Calculate the number of mipmap levels for a given size.
pub fn calc_miplevel(size: Size<u32>) -> u32 {
    let mut size = std::cmp::max(size.width, size.height);
    let mut levels = 0;
    while size != 0 {
        levels += 1;
        size >>= 1;
    }

    levels
}
