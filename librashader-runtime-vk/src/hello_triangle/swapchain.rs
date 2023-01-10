use ash::prelude::VkResult;
use ash::vk;
use crate::hello_triangle::surface::VulkanSurface;
use crate::hello_triangle::vulkan_base::VulkanBase;

pub struct VulkanSwapchain {
    pub swapchain: vk::SwapchainKHR,
    pub loader: ash::extensions::khr::Swapchain,
    pub format: vk::SurfaceFormatKHR,
    pub extent: vk::Extent2D,
    mode: vk::PresentModeKHR,
    images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    device: ash::Device,
}

impl VulkanSwapchain {
    pub fn new(base: &VulkanBase, surface: &VulkanSurface, width: u32, height:u32 ) -> VkResult<VulkanSwapchain> {
        let format = surface.choose_format(base)?;
        let mode = surface.choose_present_mode(base)?;
        let extent = surface.choose_swapchain_extent(base, width, height)?;
        let capabilities = surface.get_capabilities(base)?;

        let image_count = capabilities.min_image_count + 1;
        let image_count = if capabilities.max_image_count > 0 {
            image_count.min(capabilities.max_image_count)
        } else {
            image_count
        };

        if base.graphics_queue != surface.present_queue {
            panic!("exclusive mode only")
        }

        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.surface)
            .present_mode(mode)
            .min_image_count(image_count)
            .image_color_space(format.color_space)
            .image_format(format.format)
            .image_extent(extent)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .clipped(true)
            .image_array_layers(1)
            // todo: switch to IMAGE_USAGE_TRANSFER_DST
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .build();

        let loader = ash::extensions::khr::Swapchain::new(&base.instance, &base.device);

        let swapchain = unsafe {
            loader.create_swapchain(&create_info, None)?
        };

        let images = unsafe { loader.get_swapchain_images(swapchain)? };

        let image_views: VkResult<Vec<vk::ImageView>> = images.iter().map(|image| {
          let create_info = vk::ImageViewCreateInfo::builder()
              .view_type(vk::ImageViewType::TYPE_2D)
              .format(format.format)
              .components(vk::ComponentMapping {
                  r: vk::ComponentSwizzle::IDENTITY,
                  g: vk::ComponentSwizzle::IDENTITY,
                  b: vk::ComponentSwizzle::IDENTITY,
                  a: vk::ComponentSwizzle::IDENTITY,
              })
              .subresource_range(vk::ImageSubresourceRange {
                  aspect_mask: vk::ImageAspectFlags::COLOR,
                  base_mip_level: 0,
                  level_count: 1,
                  base_array_layer: 0,
                  layer_count: 1,
              })
              .image(*image)
              .build();


            let view = unsafe {
                base.device.create_image_view(&create_info, None)?
            };

            Ok(view)
        }).collect();
        Ok(VulkanSwapchain {
            swapchain,
            loader,
            format,
            extent,
            mode,
            images,
            image_views: image_views?,
            device: base.device.clone()
        })
    }
}

impl Drop for VulkanSwapchain {
    fn drop(&mut self) {
        unsafe {
            for view in &self.image_views {
                self.device.destroy_image_view(*view, None)
            }
            self.loader.destroy_swapchain(self.swapchain, None)
        }
    }
}