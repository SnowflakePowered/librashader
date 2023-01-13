use crate::texture::VulkanImage;

#[derive(Clone)]
pub struct Viewport<'a> {
    pub x: f32,
    pub y: f32,
    pub output: VulkanImage,
    pub mvp: Option<&'a [f32; 16]>,
}
