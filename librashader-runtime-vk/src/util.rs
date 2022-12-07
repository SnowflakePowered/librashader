use ash::vk;
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

pub fn find_vulkan_memory_type(props: &vk::PhysicalDeviceMemoryProperties, device_reqs: u32, host_reqs: vk::MemoryPropertyFlags) -> u32 {
    for i in 0..vk::MAX_MEMORY_TYPES {
        if device_reqs & (1 << i) != 0
            && props.memory_types[i].property_flags & host_reqs == host_reqs {
            return i as u32
        }
    }

    if host_reqs == vk::MemoryPropertyFlags::empty() {
        panic!("[vk] Failed to find valid memory type.")
    } else {
        find_vulkan_memory_type(props, device_reqs, vk::MemoryPropertyFlags::empty())
    }
}