use ash::vk;
use ash::vk::{Extent3D, ImageAspectFlags, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, ImageViewType, SampleCountFlags, SharingMode};
use librashader_common::Size;
use librashader_runtime::scaling::MipmapSize;
use crate::error;
use crate::renderpass::VulkanRenderPass;
use crate::util::find_vulkan_memory_type;
use crate::vulkan_primitives::VulkanImageMemory;

pub struct Framebuffer {
    device: ash::Device,
    size: Size<u32>,
    max_levels: u32,
    mem_props: vk::PhysicalDeviceMemoryProperties,
    render_pass: VulkanRenderPass,
    framebuffer: Option<VulkanFramebuffer>
}

pub struct VulkanFramebuffer {
    pub device: ash::Device,
    pub framebuffer: vk::Framebuffer,
    pub image_view: vk::ImageView,
    pub fb_view: vk::ImageView,
    pub image: vk::Image,
    pub memory: VulkanImageMemory,
}

impl Drop for VulkanFramebuffer {
    fn drop(&mut self) {
        unsafe {
            if self.framebuffer != vk::Framebuffer::null() {
                self.device.destroy_framebuffer(self.framebuffer, None);
            }
            if self.image_view != vk::ImageView::null() {
                self.device.destroy_image_view(self.image_view, None);
            }
            if self.fb_view != vk::ImageView::null() {
                self.device.destroy_image_view(self.fb_view, None);
            }
            if self.image != vk::Image::null() {
                self.device.destroy_image(self.image, None);
            }
        }
    }
}

impl Framebuffer {
    pub fn new(device: &ash::Device, size: Size<u32>, render_pass: VulkanRenderPass, mip_levels: u32, mem_props: vk::PhysicalDeviceMemoryProperties) -> error::Result<Self> {
        let mut framebuffer = Framebuffer {
            device: device.clone(),
            size,
            max_levels: mip_levels,
            mem_props,
            render_pass,
            framebuffer: None
        };

        let vulkan_image = framebuffer.create_vulkan_image()?;
        framebuffer.framebuffer = Some(vulkan_image);

        Ok(framebuffer)
    }

    pub fn create_vulkan_image(&mut self) -> error::Result<VulkanFramebuffer> {
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(ImageType::TYPE_2D)
            .format(self.render_pass.format.into())
            .extent(self.size.into())
            .mip_levels(std::cmp::min(self.max_levels, self.size.calculate_miplevels()))
            .array_layers(1)
            .samples(SampleCountFlags::TYPE_1)
            .tiling(ImageTiling::OPTIMAL)
            .usage(ImageUsageFlags::SAMPLED | ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::TRANSFER_SRC)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .initial_layout(ImageLayout::UNDEFINED)
            .build();

        let image = unsafe { self.device.create_image(&image_create_info, None)? };
        let mem_reqs = unsafe { self.device.get_image_memory_requirements(image.clone()) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_reqs.size)
            .memory_type_index(find_vulkan_memory_type(&self.mem_props, mem_reqs.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL))
            .build();

        // todo: optimize by reusing existing memory.
        let memory = VulkanImageMemory::new(&self.device, &alloc_info)?;
        memory.bind(&image)?;

        let image_subresource = vk::ImageSubresourceRange::builder()
            .base_mip_level(0)
            .base_array_layer(0)
            .level_count(image_create_info.mip_levels)
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
            .format(self.render_pass.format.into())
            .image(image.clone())
            .subresource_range(image_subresource)
            .components(swizzle_components)
            .build();

        let image_view = unsafe {
            self.device.create_image_view(&view_info, None)?
        };

        view_info.subresource_range.level_count = 1;
        let fb_view = unsafe {
            self.device.create_image_view(&view_info, None)?
        };

        let framebuffer = unsafe {
            self.device.create_framebuffer(&vk::FramebufferCreateInfo::builder()
                .render_pass(self.render_pass.render_pass)
                .attachments(&[image_view])
                .width(self.size.width)
                .height(self.size.height)
                .layers(1).build(), None)?
        };

        Ok(VulkanFramebuffer {
            device: self.device.clone(),
            framebuffer,
            memory,
            image_view,
            fb_view,
            image
        })

    }
}