use ash::vk;
use std::borrow::Cow;
use std::error::Error;

use crate::filter_chain::Vulkan;
use crate::hello_triangle::debug::VulkanDebug;
use crate::hello_triangle::physicaldevice::{find_queue_family, pick_physical_device};
use crate::hello_triangle::surface::VulkanSurface;
use ash::prelude::VkResult;
use std::ffi::{CStr, CString};

const WINDOW_TITLE: &'static str = "librashader Vulkan";

pub struct VulkanBase {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    pub debug: VulkanDebug,
    pub physical_device: vk::PhysicalDevice,
    pub mem_props: vk::PhysicalDeviceMemoryProperties,
}

impl VulkanBase {
    pub fn new(entry: ash::Entry) -> VkResult<VulkanBase> {
        let app_name = CString::new(WINDOW_TITLE).unwrap();
        let engine_name = CString::new("librashader").unwrap();

        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .engine_name(&engine_name)
            .engine_version(0)
            .application_version(0)
            .api_version(vk::make_api_version(0, 1, 3, 0))
            .build();

        // todo: make this xplat
        let extensions = [
            ash::extensions::khr::Surface::name().as_ptr(),
            ash::extensions::khr::Win32Surface::name().as_ptr(),
            ash::extensions::ext::DebugUtils::name().as_ptr(),
        ];

        let layers = unsafe {
            [CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0").as_ptr()]
        };

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .build();

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        let debug = VulkanDebug::new(&entry, &instance, Some(vulkan_debug_callback))?;

        let physical_device = pick_physical_device(&instance);

        let (device, queue) = VulkanBase::create_device(&instance, &physical_device)?;

        let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };

        Ok(VulkanBase {
            entry,
            instance,
            device,
            graphics_queue: queue,
            physical_device,
            mem_props,
            debug,
        })
    }

    fn create_device(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
    ) -> VkResult<(ash::Device, vk::Queue)> {
        let debug = unsafe {
            CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0").as_ptr()
        };

        let indices = find_queue_family(&instance, *physical_device);
        let queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(indices.graphics_family())
            .queue_priorities(&[1.0f32])
            .build();

        // let physical_device_features = vk::PhysicalDeviceFeatures::default();

        let mut physical_device_features = vk::PhysicalDeviceVulkan13Features::builder()
            .dynamic_rendering(true)
            .build();

        // let mut physical_device_features =
        //     vk::PhysicalDeviceFeatures2::builder().push_next(&mut physical_device_features)
        //         .build();


        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&[queue_info])
            .enabled_layer_names(&[debug])
            .enabled_extension_names(&[
                ash::extensions::khr::Swapchain::name().as_ptr(),
                ash::extensions::khr::DynamicRendering::name().as_ptr(),
            ])
            .push_next(&mut physical_device_features)
            // .enabled_features(&physical_device_features)
            .build();

        let device =
            unsafe { instance.create_device(*physical_device, &device_create_info, None)? };

        let queue = unsafe { device.get_device_queue(indices.graphics_family(), 0) };

        Ok((device, queue))
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}

impl Drop for VulkanBase {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}

impl TryFrom<&VulkanBase> for Vulkan {
    type Error = Box<dyn Error>;

    fn try_from(value: &VulkanBase) -> Result<Self, Self::Error> {
        Vulkan::try_from((
            value.device.clone(),
            value.graphics_queue.clone(),
            value.mem_props,
        ))
    }
}
