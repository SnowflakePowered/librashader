use crate::error;
use crate::memory::VulkanBuffer;
use ash::vk;
use gpu_allocator::vulkan::Allocator;
use librashader_runtime::quad::QuadType;
use parking_lot::RwLock;
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
        allocator: &Arc<RwLock<Allocator>>,
    ) -> error::Result<DrawQuad> {
        let mut buffer = VulkanBuffer::new(
            device,
            allocator,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            2 * std::mem::size_of::<[f32; 16]>(),
        )?;

        {
            let slice = buffer.as_mut_slice()?;
            slice[0..std::mem::size_of::<[f32; 16]>()]
                .copy_from_slice(bytemuck::cast_slice(VBO_OFFSCREEN));
            slice[std::mem::size_of::<[f32; 16]>()..]
                .copy_from_slice(bytemuck::cast_slice(VBO_DEFAULT_FINAL));
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
