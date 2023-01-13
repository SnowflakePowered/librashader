use crate::{error, util};
use ash::vk;
use std::ffi::c_void;

pub struct VulkanImageMemory {
    pub handle: vk::DeviceMemory,
    device: ash::Device,
}

impl VulkanImageMemory {
    pub fn new(
        device: &ash::Device,
        alloc: &vk::MemoryAllocateInfo,
    ) -> error::Result<VulkanImageMemory> {
        unsafe {
            Ok(VulkanImageMemory {
                handle: device.allocate_memory(alloc, None)?,
                device: device.clone(),
            })
        }
    }

    pub fn bind(&self, image: &vk::Image) -> error::Result<()> {
        unsafe {
            Ok(self
                .device
                .bind_image_memory(image.clone(), self.handle.clone(), 0)?)
        }
    }
}

impl Drop for VulkanImageMemory {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.handle, None);
        }
    }
}

pub struct VulkanBuffer {
    pub handle: vk::Buffer,
    device: ash::Device,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
}

pub struct VulkanBufferMapHandle<'a> {
    buffer: &'a mut VulkanBuffer,
    ptr: *mut c_void,
}

impl VulkanBuffer {
    pub fn new(
        device: &ash::Device,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
        usage: vk::BufferUsageFlags,
        size: usize,
    ) -> error::Result<VulkanBuffer> {
        unsafe {
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(size as vk::DeviceSize)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build();
            let buffer = device.create_buffer(&buffer_info, None)?;

            let memory_reqs = device.get_buffer_memory_requirements(buffer);
            let alloc_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(memory_reqs.size)
                .memory_type_index(util::find_vulkan_memory_type(
                    mem_props,
                    memory_reqs.memory_type_bits,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                ))
                .build();

            let alloc = device.allocate_memory(&alloc_info, None)?;
            device.bind_buffer_memory(buffer.clone(), alloc.clone(), 0)?;

            Ok(VulkanBuffer {
                handle: buffer,
                memory: alloc,
                size: size as vk::DeviceSize,
                device: device.clone(),
            })
        }
    }

    pub fn map(&mut self) -> error::Result<VulkanBufferMapHandle> {
        let dst = unsafe {
            self.device
                .map_memory(self.memory, 0, self.size, vk::MemoryMapFlags::empty())?
        };

        Ok(VulkanBufferMapHandle {
            buffer: self,
            ptr: dst,
        })
    }
}

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            if self.memory != vk::DeviceMemory::null() {
                self.device.free_memory(self.memory, None);
            }

            if self.handle != vk::Buffer::null() {
                self.device.destroy_buffer(self.handle, None);
            }
        }
    }
}

impl<'a> VulkanBufferMapHandle<'a> {
    pub unsafe fn copy_from(&mut self, offset: usize, src: &[u8]) {
        if self.buffer.size > (offset + src.len()) as u64 {
            panic!("invalid write")
        }
        std::ptr::copy_nonoverlapping(
            src.as_ptr(),
            self.ptr
                .map_addr(|original| original.wrapping_add(offset))
                .cast(),
            src.len(),
        );
    }
}

impl<'a> Drop for VulkanBufferMapHandle<'a> {
    fn drop(&mut self) {
        unsafe { self.buffer.device.unmap_memory(self.buffer.memory) }
    }
}
