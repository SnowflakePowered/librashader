use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub struct WgpuMappedBuffer {
    buffer: wgpu::Buffer,
    shadow: Box<[u8]>,
}

impl WgpuMappedBuffer {
    pub fn new(
        device: &Arc<wgpu::Device>,
        usage: wgpu::BufferUsages,
        size: wgpu::BufferAddress,
        label: wgpu::Label<'static>
    ) -> WgpuMappedBuffer {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size,
            usage,
            mapped_at_creation: true,
        });

        WgpuMappedBuffer {
            buffer,
            shadow: vec![0u8; size as usize].into_boxed_slice(),
        }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Write the contents of the backing buffer to the device buffer.
    pub fn flush(&self) {
        self.buffer.slice(..)
            .get_mapped_range_mut().copy_from_slice(&self.shadow)
    }
}

impl Deref for WgpuMappedBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.shadow.deref()
    }
}

impl DerefMut for WgpuMappedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.shadow.deref_mut()
    }
}