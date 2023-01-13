use crate::framebuffer::OutputImage;

#[rustfmt::skip]
pub(crate) static DEFAULT_MVP: &[f32; 16] = &[
    2f32, 0.0, 0.0, 0.0,
    0.0, 2.0, 0.0, 0.0,
    0.0, 0.0, 2.0, 0.0,
    -1.0, -1.0, 0.0, 1.0,
];

#[derive(Clone)]
pub(crate) struct RenderTarget<'a> {
    pub x: f32,
    pub y: f32,
    pub mvp: &'a [f32; 16],
    pub output: OutputImage,
}
