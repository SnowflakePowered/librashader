use ash::vk;
use librashader_runtime::ringbuffer::{BoxRingBuffer, InlineRingBuffer};
use crate::error;
use crate::vulkan_primitives::VulkanBuffer;

pub struct VkUboRing {
    ring: BoxRingBuffer<VulkanBuffer>,
}

impl VkUboRing {
    pub fn new(device: &ash::Device, mem_props: &vk::PhysicalDeviceMemoryProperties, ring_size: usize, buffer_size: usize) -> error::Result<Self> {
        let mut ring = Vec::new();
        for _ in 0..ring_size {
            ring.push(VulkanBuffer::new(device, mem_props, vk::BufferUsageFlags::UNIFORM_BUFFER, buffer_size)?);
        }

        Ok(VkUboRing {
            ring: BoxRingBuffer::from_vec(ring)
        })
    }
}
