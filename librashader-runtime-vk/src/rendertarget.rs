use crate::framebuffer::OutputFramebuffer;
use ash::vk;

#[rustfmt::skip]
pub static DEFAULT_MVP: &[f32; 16] = &[
    2f32, 0.0, 0.0, 0.0,
    0.0, 2.0, 0.0, 0.0,
    0.0, 0.0, 2.0, 0.0,
    -1.0, -1.0, 0.0, 1.0,
];

#[derive(Clone)]
pub(crate) struct RenderTarget<'a> {
    pub mvp: &'a [f32; 16],
    pub output: OutputFramebuffer,
}
