use crate::error;
use crate::renderpass::VulkanRenderPass;
use ash::vk;
use ash::vk::{
    ImageAspectFlags,
    ImageViewType,
};
use librashader_common::Size;
use crate::filter_chain::Vulkan;
use crate::texture::OwnedTexture;

pub struct VulkanFramebuffer {
    pub device: ash::Device,
    pub framebuffer: vk::Framebuffer,
}

impl Drop for VulkanFramebuffer {
    fn drop(&mut self) {
        unsafe {
            if self.framebuffer != vk::Framebuffer::null() {
                self.device.destroy_framebuffer(self.framebuffer, None);
            }
        }
    }
}

pub struct OwnedFramebuffer {
    pub size: Size<u32>,
    pub image: OwnedTexture,
    pub render_pass: VulkanRenderPass,
    pub framebuffer: VulkanFramebuffer,
    pub view: vk::ImageView,
}

impl OwnedFramebuffer {
    pub fn new(
        vulkan: &Vulkan,
        size: Size<u32>,
        render_pass: VulkanRenderPass,
        max_miplevels: u32,
    ) -> error::Result<Self> {
        let image = OwnedTexture::new(vulkan, size, render_pass.format, max_miplevels)?;
        let fb_view = image.create_texture_view()?;
        let framebuffer = unsafe {
            vulkan.device.create_framebuffer(
                &vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass.handle)
                    .attachments(&[image.image_view])
                    .width(image.image.size.width)
                    .height(image.image.size.height)
                    .layers(1)
                    .build(),
                None,
            )?
        };

        Ok(OwnedFramebuffer {
            size,
            image,
            view: fb_view,
            render_pass,
            framebuffer: VulkanFramebuffer {
                device: vulkan.device.clone(),
                framebuffer,
            },
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OutputFramebuffer {
    pub framebuffer: vk::Framebuffer,
    pub size: Size<u32>,
    pub viewport: vk::Viewport,
}


//
// pub struct OutputFramebuffer<'a> {
//     device: ash::Device,
//     render_pass: &'a VulkanRenderPass,
//     pub handle: vk::Framebuffer,
//     pub size: Size<u32>,
//     pub image: vk::Image,
//     pub image_view: vk::ImageView,
// }
//
// impl<'a> OutputFramebuffer<'a> {
//     pub fn new(vulkan: &Vulkan, render_pass: &'a VulkanRenderPass, image: vk::Image, size: Size<u32>) -> error::Result<OutputFramebuffer<'a>> {
//         let image_subresource = vk::ImageSubresourceRange::builder()
//             .base_mip_level(0)
//             .base_array_layer(0)
//             .level_count(1)
//             .layer_count(1)
//             .aspect_mask(ImageAspectFlags::COLOR)
//             .build();
//
//         let swizzle_components = vk::ComponentMapping::builder()
//             .r(vk::ComponentSwizzle::R)
//             .g(vk::ComponentSwizzle::G)
//             .b(vk::ComponentSwizzle::B)
//             .a(vk::ComponentSwizzle::A)
//             .build();
//
//         let mut view_info = vk::ImageViewCreateInfo::builder()
//             .view_type(ImageViewType::TYPE_2D)
//             .format(render_pass.format.into())
//             .image(image.clone())
//             .subresource_range(image_subresource)
//             .components(swizzle_components)
//             .build();
//
//         let image_view = unsafe { vulkan.device.create_image_view(&view_info, None)? };
//
//         let framebuffer = unsafe {
//             vulkan.device.create_framebuffer(
//                 &vk::FramebufferCreateInfo::builder()
//                     .render_pass(render_pass.handle)
//                     .attachments(&[image_view])
//                     .width(size.width)
//                     .height(size.height)
//                     .layers(1)
//                     .build(),
//                 None,
//             )?
//         };
//
//         Ok(OutputFramebuffer {
//             device: vulkan.device.clone(),
//             image,
//             image_view,
//             render_pass,
//             size,
//             handle: framebuffer,
//         })
//     }
//
//     pub fn get_renderpass_begin_info(&self, area: vk::Rect2D, clear: Option<&[vk::ClearValue]>) -> vk::RenderPassBeginInfo {
//         let mut builder = vk::RenderPassBeginInfo::builder()
//             .render_pass(self.render_pass.handle)
//             .framebuffer(self.handle)
//             .render_area(area);
//
//         if let Some(clear) = clear {
//             builder = builder.clear_values(clear)
//         }
//
//         builder.build()
//     }
// }