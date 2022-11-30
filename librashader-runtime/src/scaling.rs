use librashader_common::Size;
use librashader_presets::{Scale2D, ScaleFactor, ScaleType, Scaling};
use num_traits::AsPrimitive;
use std::ops::Mul;

pub fn scale<T>(scaling: Scale2D, source: Size<T>, viewport: Size<T>) -> Size<T>
where
    T: Mul<ScaleFactor, Output = f32> + Copy + 'static,
    f32: AsPrimitive<T>,
{
    let width: f32;
    let height: f32;

    match scaling.x {
        Scaling {
            scale_type: ScaleType::Input,
            factor,
        } => width = source.width * factor,
        Scaling {
            scale_type: ScaleType::Absolute,
            factor,
        } => width = factor.into(),
        Scaling {
            scale_type: ScaleType::Viewport,
            factor,
        } => width = viewport.width * factor,
    };

    match scaling.y {
        Scaling {
            scale_type: ScaleType::Input,
            factor,
        } => height = source.height * factor,
        Scaling {
            scale_type: ScaleType::Absolute,
            factor,
        } => height = factor.into(),
        Scaling {
            scale_type: ScaleType::Viewport,
            factor,
        } => height = viewport.height * factor,
    };

    Size {
        width: width.round().as_(),
        height: height.round().as_(),
    }
}

pub fn calc_miplevel(size: Size<u32>) -> u32 {
    let mut size = std::cmp::max(size.width, size.height);
    let mut levels = 0;
    while size != 0 {
        levels += 1;
        size >>= 1;
    }

    levels
}
