use crate::error;
use crate::vulkan_primitives::VulkanBuffer;
use ash::vk;

#[rustfmt::skip]
pub(crate) static VBO_OFFSCREEN: &[f32; 16] = &[
    // Offscreen
    -1.0, -1.0, 0.0, 0.0,
    -1.0, 1.0, 0.0, 1.0,
    1.0, -1.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];

#[rustfmt::skip]
pub(crate) static VBO_DEFAULT_FINAL: &[f32; 16] = &[
    // Final
    0.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 1.0,
    1.0, 0.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];

pub enum VboType {
    Offscreen,
    Final,
}

pub struct DrawQuad {
    buffer: VulkanBuffer,
    device: ash::Device,
}

impl DrawQuad {
    pub fn new(
        device: &ash::Device,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
    ) -> error::Result<DrawQuad> {
        let mut buffer = VulkanBuffer::new(
            device,
            mem_props,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            2 * std::mem::size_of::<[f32; 16]>(),
        )?;

        {
            let mut map = buffer.map()?;
            unsafe {
                map.copy_from(0, bytemuck::cast_slice(VBO_OFFSCREEN));
                map.copy_from(
                    std::mem::size_of::<[f32; 16]>(),
                    bytemuck::cast_slice(VBO_DEFAULT_FINAL),
                );
            }
        }
        Ok(DrawQuad {
            buffer,
            device: device.clone(),
        })
    }

    pub fn bind_vbo(&self, cmd: vk::CommandBuffer, vbo: VboType) {
        let offset = match vbo {
            VboType::Offscreen => 0,
            VboType::Final => std::mem::size_of::<[f32; 16]>(),
        };

        unsafe {
            self.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.buffer.handle],
                &[offset as vk::DeviceSize],
            )
        }
    }
}
