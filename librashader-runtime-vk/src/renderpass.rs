use crate::error;
use ash::vk;
use ash::vk::{
    AttachmentLoadOp, AttachmentStoreOp, ImageLayout, PipelineBindPoint, SampleCountFlags,
};
use librashader_common::ImageFormat;

pub struct VulkanRenderPass {
    pub handle: vk::RenderPass,
    pub format: ImageFormat,
}

impl VulkanRenderPass {
    pub fn create_render_pass(
        device: &ash::Device,
        mut format: ImageFormat,
    ) -> error::Result<Self> {
        // default to reasonable choice if unknown
        if format == ImageFormat::Unknown {
            format = ImageFormat::R8G8B8A8Unorm;
        }

        let attachment = vk::AttachmentDescription::builder()
            .flags(vk::AttachmentDescriptionFlags::empty())
            .format(format.into())
            .samples(SampleCountFlags::TYPE_1)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
            .initial_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .final_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
            .color_attachments(&[attachment_ref])
            .build();

        let renderpass_info = vk::RenderPassCreateInfo::builder()
            .flags(vk::RenderPassCreateFlags::empty())
            .attachments(&[attachment])
            .subpasses(&[subpass])
            .build();

        unsafe {
            let rp = device.create_render_pass(&renderpass_info, None)?;
            Ok(Self { handle: rp, format })
        }
    }
}
