use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub struct WgpuMappedBuffer {
    buffer: wgpu::Buffer,
    backing: Box<[u8]>,
    device: Arc<wgpu::Device>
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
            backing: vec![0u8; size as usize].into_boxed_slice(),
            device: Arc::clone(&device)
        }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Write the contents of the backing buffer to the device buffer.
    pub fn flush(&self) {
        self.buffer.slice(..)
            .get_mapped_range_mut().copy_from_slice(&self.backing)
    }
}

impl Deref for WgpuMappedBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.backing.deref()
    }
}

impl DerefMut for WgpuMappedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.backing.deref_mut()
    }
}