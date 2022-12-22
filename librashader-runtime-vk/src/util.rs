use crate::error;
use ash::vk;
use ash::vk::{AccessFlags, Extent3D, ImageAspectFlags};
use librashader_common::Size;
use librashader_reflect::reflect::semantics::BindingStage;

pub fn binding_stage_to_vulkan_stage(stage_mask: BindingStage) -> vk::ShaderStageFlags {
    let mut mask = vk::ShaderStageFlags::default();
    if stage_mask.contains(BindingStage::VERTEX) {
        mask |= vk::ShaderStageFlags::VERTEX;
    }

    if stage_mask.contains(BindingStage::FRAGMENT) {
        mask |= vk::ShaderStageFlags::FRAGMENT;
    }

    mask
}

pub fn find_vulkan_memory_type(
    props: &vk::PhysicalDeviceMemoryProperties,
    device_reqs: u32,
    host_reqs: vk::MemoryPropertyFlags,
) -> u32 {
    for i in 0..vk::MAX_MEMORY_TYPES {
        if device_reqs & (1 << i) != 0
            && props.memory_types[i].property_flags & host_reqs == host_reqs
        {
            return i as u32;
        }
    }

    if host_reqs == vk::MemoryPropertyFlags::empty() {
        panic!("[vk] Failed to find valid memory type.")
    } else {
        find_vulkan_memory_type(props, device_reqs, vk::MemoryPropertyFlags::empty())
    }
}

#[inline(always)]
pub unsafe fn vulkan_image_layout_transition_levels(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    levels: u32,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    src_access: vk::AccessFlags,
    dst_access: vk::AccessFlags,
    src_stage: vk::PipelineStageFlags,
    dst_stage: vk::PipelineStageFlags,

    src_queue_family_index: u32,
    dst_queue_family_index: u32,
) {
    let mut barrier = vk::ImageMemoryBarrier::default();
    barrier.s_type = vk::StructureType::IMAGE_MEMORY_BARRIER;
    barrier.p_next = std::ptr::null();
    barrier.src_access_mask = src_access;
    barrier.dst_access_mask = dst_access;
    barrier.old_layout = old_layout;
    barrier.new_layout = new_layout;
    barrier.src_queue_family_index = src_queue_family_index;
    barrier.dst_queue_family_index = dst_queue_family_index;
    barrier.image = image.clone();
    barrier.subresource_range.aspect_mask = vk::ImageAspectFlags::COLOR;
    barrier.subresource_range.base_array_layer = 0;
    barrier.subresource_range.level_count = levels;
    barrier.subresource_range.layer_count = vk::REMAINING_ARRAY_LAYERS;
    device.cmd_pipeline_barrier(
        cmd,
        src_stage,
        dst_stage,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[barrier],
    )
}
