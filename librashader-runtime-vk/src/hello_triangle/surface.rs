use ash::prelude::VkResult;
use ash::vk;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use crate::hello_triangle::vulkan_base::VulkanBase;

pub struct VulkanSurface {
    surface_loader: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR,
    present_queue: vk::Queue
}

impl VulkanSurface {
    pub fn new(base: &VulkanBase,
               window: &winit::window::Window) -> VkResult<VulkanSurface>
    {
        let surface = unsafe {
            ash_window::create_surface(
                &base.entry,
                &base.instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )?
        };

        let surface_loader = ash::extensions::khr::Surface::new(&base.entry, &base.instance);

        let present_queue = unsafe {
            let queue_family = base.instance
                .get_physical_device_queue_family_properties(base.physical_device)
                .iter()
                .enumerate()
                .find_map(|(index, info)| {
                    let supports_graphic_and_surface =
                        info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                            && surface_loader
                            .get_physical_device_surface_support(
                                base.physical_device,
                                index as u32,
                                surface,
                            )
                            .unwrap();
                    if supports_graphic_and_surface {
                        Some(index)
                    } else {
                        None
                    }
                })
                .expect("couldn't find suitable device");
            base.device.get_device_queue(queue_family as u32, 0)
        };

        Ok(VulkanSurface {
            surface,
            surface_loader,
            present_queue
        })
    }
}

