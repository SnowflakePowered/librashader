use ash::vk;
use crate::framebuffer::VulkanFramebuffer;

pub struct RenderTarget {
    pub mvp: [f32; 16],
    pub image: vk::Image,
    pub framebuffer: VulkanFramebuffer
}