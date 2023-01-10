use crate::{error, util};
use crate::filter_chain::Vulkan;
use crate::renderpass::VulkanRenderPass;
use crate::texture::OwnedTexture;
use ash::vk;
use ash::vk::{ImageAspectFlags, ImageViewType};
use librashader_common::Size;

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
//
// pub struct OwnedFramebuffer {
//     pub size: Size<u32>,
//     pub image: OwnedTexture,
//     pub render_pass: VulkanRenderPass,
//     pub framebuffer: VulkanFramebuffer,
//     pub view: vk::ImageView,
// }
//
// impl OwnedFramebuffer {
//     pub fn new(
//         vulkan: &Vulkan,
//         size: Size<u32>,
//         render_pass: VulkanRenderPass,
//         max_miplevels: u32,
//     ) -> error::Result<Self> {
//         let image = OwnedTexture::new(vulkan, size, render_pass.format, max_miplevels)?;
//         let fb_view = image.create_texture_view()?;
//         let framebuffer = unsafe {
//             vulkan.device.create_framebuffer(
//                 &vk::FramebufferCreateInfo::builder()
//                     .render_pass(render_pass.handle)
//                     .attachments(&[image.image_view])
//                     .width(image.image.size.width)
//                     .height(image.image.size.height)
//                     .layers(1)
//                     .build(),
//                 None,
//             )?
//         };
//
//         Ok(OwnedFramebuffer {
//             size,
//             image,
//             view: fb_view,
//             render_pass,
//             framebuffer: VulkanFramebuffer {
//                 device: vulkan.device.clone(),
//                 framebuffer,
//             },
//         })
//     }
// }

#[derive(Clone)]
pub(crate) struct OutputFramebuffer {
    pub framebuffer: vk::Framebuffer,
    pub size: Size<u32>,
    device: ash::Device,
    image_view: vk::ImageView,
    image: vk::Image,
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
impl OutputFramebuffer {
    pub fn new(vulkan: &Vulkan, render_pass: &VulkanRenderPass, image: vk::Image, size: Size<u32>) -> error::Result<OutputFramebuffer> {
        let image_subresource = vk::ImageSubresourceRange::builder()
            .base_mip_level(0)
            .base_array_layer(0)
            .level_count(1)
            .layer_count(1)
            .aspect_mask(ImageAspectFlags::COLOR)
            .build();

        let swizzle_components = vk::ComponentMapping::builder()
            .r(vk::ComponentSwizzle::R)
            .g(vk::ComponentSwizzle::G)
            .b(vk::ComponentSwizzle::B)
            .a(vk::ComponentSwizzle::A)
            .build();

        let mut view_info = vk::ImageViewCreateInfo::builder()
            .view_type(ImageViewType::TYPE_2D)
            .format(render_pass.format.into())
            .image(image.clone())
            .subresource_range(image_subresource)
            .components(swizzle_components)
            .build();

        let image_view = unsafe { vulkan.device.create_image_view(&view_info, None)? };

        let framebuffer = unsafe {
            vulkan.device.create_framebuffer(
                &vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass.handle)
                    .attachments(&[image_view])
                    .width(size.width)
                    .height(size.height)
                    .layers(1)
                    .build(),
                None,
            )?
        };

        Ok(OutputFramebuffer {
            device: vulkan.device.clone(),
            size,
            image,
            framebuffer,
            image_view,
        })
    }

    pub fn begin_pass(&self, cmd: vk::CommandBuffer) {
        unsafe {
            util::vulkan_image_layout_transition_levels(&self.device, cmd, self.image,
                                                        1,
                                                        vk::ImageLayout::UNDEFINED,
                                                        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                                                        vk::AccessFlags::empty(),
                                                        vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::ALL_GRAPHICS,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::QUEUE_FAMILY_IGNORED,
            vk::QUEUE_FAMILY_IGNORED)
        }
    }

    pub fn end_pass(&self, cmd: vk::CommandBuffer) {
        // todo: generate mips
        unsafe {
            util::vulkan_image_layout_transition_levels(&self.device, cmd, self.image,
                                                        vk::REMAINING_MIP_LEVELS,
                                                        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                                                        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                                                        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                                                        vk::AccessFlags::SHADER_READ,
                                                        vk::PipelineStageFlags::ALL_GRAPHICS,
                                                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                                                        vk::QUEUE_FAMILY_IGNORED,
                                                        vk::QUEUE_FAMILY_IGNORED)
        }
    }

    // pub fn get_renderpass_begin_info(&self, area: vk::Rect2D, clear: Option<&[vk::ClearValue]>) -> vk::RenderPassBeginInfo {
    //     let mut builder = vk::RenderPassBeginInfo::builder()
    //         .render_pass(self.render_pass.handle)
    //         .framebuffer(self.handle)
    //         .render_area(area);
    //
    //     if let Some(clear) = clear {
    //         builder = builder.clear_values(clear)
    //     }
    //
    //     builder.build()
    // }
}

impl Drop for OutputFramebuffer {
    fn drop(&mut self) {
        unsafe {
            if self.framebuffer != vk::Framebuffer::null() {
                self.device.destroy_framebuffer(self.framebuffer, None);
            }
            if self.image_view != vk::ImageView::null() {
                self.device.destroy_image_view(self.image_view, None);
            }
        }
    }
}