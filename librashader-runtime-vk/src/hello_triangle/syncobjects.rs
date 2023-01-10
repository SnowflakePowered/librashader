use ash::prelude::VkResult;
use ash::vk;

pub struct SyncObjects {
    pub image_available: vk::Semaphore,
    pub render_finished: vk::Semaphore,
    pub in_flight: vk::Fence
}

impl SyncObjects {
    pub fn new(device: &ash::Device) -> VkResult<SyncObjects> {
        unsafe {
            let image_available = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;
            let render_finished = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;
            let in_flight = device.create_fence(&vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED).build(), None)?;

            Ok(SyncObjects {
                image_available,
                render_finished,
                in_flight
            })
        }
    }
}