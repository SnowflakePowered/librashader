use crate::scaling;
use librashader_common::Size;
use librashader_presets::{Scale2D, ScaleFactor, ScaleType, Scaling};
use num_traits::AsPrimitive;
use std::ops::Mul;

pub trait ViewportSize<T>
where
    T: Mul<ScaleFactor, Output = f32> + Copy + 'static,
    f32: AsPrimitive<T>,
{
    /// Produce a `Size<T>` scaled with the input scaling options.
    fn scale_viewport(self, scaling: Scale2D, viewport: Size<T>) -> Size<T>;
}

impl<T> ViewportSize<T> for Size<T>
where
    T: Mul<ScaleFactor, Output = f32> + Copy + 'static,
    f32: AsPrimitive<T>,
{
    fn scale_viewport(self, scaling: Scale2D, viewport: Size<T>) -> Size<T>
    where
        T: Mul<ScaleFactor, Output = f32> + Copy + 'static,
        f32: AsPrimitive<T>,
    {
        scaling::scale(scaling, self, viewport)
    }
}

pub trait MipmapSize<T> {
    /// Calculate the number of mipmap levels for a given size.
    fn calculate_miplevels(self) -> T;

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

fn scale<T>(scaling: Scale2D, source: Size<T>, viewport: Size<T>) -> Size<T>
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

pub fn calculate_miplevels(size: Size<u32>) -> u32 {
    let mut size = std::cmp::max(size.width, size.height);
    let mut levels = 0;
    while size != 0 {
        levels += 1;
        size >>= 1;
    }

    levels
}
