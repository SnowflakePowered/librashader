use ash::vk;
use crate::framebuffer::{OutputFramebuffer,};

#[derive(Debug, Clone)]
pub(crate) struct RenderTarget<'a> {
    pub mvp: &'a [f32; 16],
    pub output: OutputFramebuffer,
}