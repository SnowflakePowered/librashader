use crate::{error, util};
use ash::vk;
use librashader_runtime::uniforms::UniformStorageAccess;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::Arc;

pub struct VulkanImageMemory {
    pub handle: vk::DeviceMemory,
    device: Arc<ash::Device>,
}

impl VulkanImageMemory {
    pub fn new(
        device: &Arc<ash::Device>,
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
        unsafe { Ok(self.device.bind_image_memory(*image, self.handle, 0)?) }
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
    device: Arc<ash::Device>,
    memory: vk::DeviceMemory,
    size: vk::DeviceSize,
}

pub struct VulkanBufferMapHandle<'a> {
    buffer: &'a mut VulkanBuffer,
    ptr: *mut c_void,
}

impl VulkanBuffer {
    pub fn new(
        device: &Arc<ash::Device>,
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
                )?)
                .build();

            let alloc = device.allocate_memory(&alloc_info, None)?;
            device.bind_buffer_memory(buffer, alloc, 0)?;

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
        let mut align = ash::util::Align::new(
            self.ptr
                .map_addr(|original| original.wrapping_add(offset))
                .cast(),
            std::mem::align_of::<u8>() as u64,
            self.buffer.size,
        );

        align.copy_from_slice(src);
    }
}

impl<'a> Drop for VulkanBufferMapHandle<'a> {
    fn drop(&mut self) {
        unsafe { self.buffer.device.unmap_memory(self.buffer.memory) }
    }
}

/// SAFETY: Creating the pointer should be safe in multithreaded contexts.
///
/// Mutation is guarded by DerefMut<Target=[u8]>
unsafe impl Send for RawVulkanBuffer {}
pub struct RawVulkanBuffer {
    buffer: ManuallyDrop<VulkanBuffer>,
    ptr: NonNull<c_void>,
}

impl RawVulkanBuffer {
    pub fn new(
        device: &Arc<ash::Device>,
        mem_props: &vk::PhysicalDeviceMemoryProperties,
        usage: vk::BufferUsageFlags,
        size: usize,
    ) -> error::Result<Self> {
        let buffer = ManuallyDrop::new(VulkanBuffer::new(device, mem_props, usage, size)?);
        let ptr = unsafe {
            NonNull::new_unchecked(device.map_memory(
                buffer.memory,
                0,
                buffer.size,
                vk::MemoryMapFlags::empty(),
            )?)
        };

        Ok(RawVulkanBuffer { buffer, ptr })
    }

    pub fn bind_to_descriptor_set(
        &self,
        descriptor_set: vk::DescriptorSet,
        binding: u32,
        storage: &impl UniformStorageAccess,
    ) -> error::Result<()> {
        unsafe {
            let buffer_info = [vk::DescriptorBufferInfo::builder()
                .buffer(self.buffer.handle)
                .offset(0)
                .range(storage.ubo_slice().len() as vk::DeviceSize)
                .build()];

            let write_info = [vk::WriteDescriptorSet::builder()
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .dst_set(descriptor_set)
                .dst_binding(binding)
                .dst_array_element(0)
                .buffer_info(&buffer_info)
                .build()];

            self.buffer.device.update_descriptor_sets(&write_info, &[])
        }
        Ok(())
    }
}

impl Drop for RawVulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            self.buffer.device.unmap_memory(self.buffer.memory);
            self.ptr = NonNull::dangling();
            if self.buffer.memory != vk::DeviceMemory::null() {
                self.buffer.device.free_memory(self.buffer.memory, None);
            }

            if self.buffer.handle != vk::Buffer::null() {
                self.buffer.device.destroy_buffer(self.buffer.handle, None);
            }
        }
    }
}

impl Deref for RawVulkanBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr().cast(), self.buffer.size as usize) }
    }
}

impl DerefMut for RawVulkanBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.as_ptr().cast(), self.buffer.size as usize)
        }
    }
}
