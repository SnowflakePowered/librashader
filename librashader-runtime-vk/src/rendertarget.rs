use crate::framebuffer::OutputFramebuffer;
use ash::vk;

#[derive(Clone)]
pub(crate) struct RenderTarget<'a> {
    pub mvp: &'a [f32; 16],
    pub output: OutputFramebuffer,
}
