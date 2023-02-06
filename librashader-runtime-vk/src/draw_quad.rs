use crate::error;
use crate::vulkan_primitives::VulkanBuffer;
use ash::vk;
use librashader_runtime::quad::QuadType;
use std::sync::Arc;

#[rustfmt::skip]
static VBO_OFFSCREEN: &[f32; 16] = &[
    // Offscreen
    -1.0, -1.0, 0.0, 0.0,
    -1.0, 1.0, 0.0, 1.0,
    1.0, -1.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];

#[rustfmt::skip]
static VBO_DEFAULT_FINAL: &[f32; 16] = &[
    // Final
    0.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 1.0,
    1.0, 0.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];

pub struct DrawQuad {
    buffer: VulkanBuffer,
    device: Arc<ash::Device>,
}

impl DrawQuad {
    pub fn new(
        device: &Arc<ash::Device>,
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

    pub fn bind_vbo_for_frame(&self, cmd: vk::CommandBuffer) {
        unsafe {
            self.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.buffer.handle],
                &[0 as vk::DeviceSize],
            )
        }
    }

    pub fn draw_quad(&self, cmd: vk::CommandBuffer, vbo: QuadType) {
        let offset = match vbo {
            QuadType::Offscreen => 0,
            QuadType::Final => 4,
        };

        unsafe {
            self.device.cmd_draw(cmd, 4, 1, offset, 0);
        }
    }
}
