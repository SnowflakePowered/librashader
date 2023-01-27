use crate::ctypes::{libra_shader_preset_t, libra_viewport_t, libra_vk_filter_chain_t};
use crate::error::{assert_non_null, assert_some_ptr, LibrashaderError};
use crate::ffi::extern_fn;
use librashader::runtime::vk::{VulkanImage, VulkanInstance};
use std::ffi::CStr;
use std::ffi::{c_char, c_void};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::slice;

pub use librashader::runtime::vk::capi::options::FilterChainOptionsVulkan;
pub use librashader::runtime::vk::capi::options::FrameOptionsVulkan;
use librashader::runtime::FilterChainParameters;
use librashader::runtime::{Size, Viewport};

use ash::vk;

pub use ash::vk::PFN_vkGetInstanceProcAddr;

/// A Vulkan instance function loader that the Vulkan filter chain needs to be initialized with.
pub type libra_PFN_vkGetInstanceProcAddr =
    unsafe extern "system" fn(instance: *mut c_void, p_name: *const c_char);

/// Vulkan parameters for the source image.
#[repr(C)]
pub struct libra_image_vk_t {
    /// A raw `VkImage` handle to the source image.
    pub handle: vk::Image,
    /// The `VkFormat` of the source image.
    pub format: vk::Format,
    /// The width of the source image.
    pub width: u32,
    /// The height of the source image.
    pub height: u32,
}

/// Handles required to instantiate vulkan
#[repr(C)]
pub struct libra_device_vk_t {
    /// A raw `VkPhysicalDevice` handle
    /// for the physical device that will perform rendering.
    pub physical_device: vk::PhysicalDevice,
    /// A raw `VkInstance` handle
    /// for the Vulkan instance that will perform rendering.
    pub instance: vk::Instance,
    /// A raw `VkDevice` handle
    /// for the device attached to the instance that will perform rendering.
    pub device: vk::Device,
    /// The entry loader for the Vulkan library.
    pub entry: vk::PFN_vkGetInstanceProcAddr,
}

impl From<libra_image_vk_t> for VulkanImage {
    fn from(value: libra_image_vk_t) -> Self {
        VulkanImage {
            size: Size::new(value.width, value.height),
            image: value.handle,
            format: value.format,
        }
    }
}

impl From<libra_device_vk_t> for VulkanInstance {
    fn from(value: libra_device_vk_t) -> Self {
        VulkanInstance {
            device: value.device,
            instance: value.instance,
            physical_device: value.physical_device,
            get_instance_proc_addr: value.entry,
        }
    }
}

extern_fn! {
    /// Create the filter chain given the shader preset.
    ///
    /// The shader preset is immediately invalidated and must be recreated after
    /// the filter chain is created.
    ///
    /// ## Safety:
    /// - The handles provided in `vulkan` must be valid for the command buffers that
    ///   `libra_vk_filter_chain_frame` will write to. Namely, the VkDevice must have been
    ///    created with the `VK_KHR_dynamic_rendering` extension.
    /// - `preset` must be either null, or valid and aligned.
    /// - `options` must be either null, or valid and aligned.
    /// - `out` must be aligned, but may be null, invalid, or uninitialized.
    fn libra_vk_filter_chain_create(
        vulkan: libra_device_vk_t,
        preset: *mut libra_shader_preset_t,
        options: *const FilterChainOptionsVulkan,
        out: *mut MaybeUninit<libra_vk_filter_chain_t>
    ) {
        assert_non_null!(preset);
        let preset = unsafe {
            let preset_ptr = &mut *preset;
            let preset = preset_ptr.take();
            Box::from_raw(preset.unwrap().as_ptr())
        };

        let options = if options.is_null() {
            None
        } else {
            Some(unsafe { &*options })
        };

        let vulkan: VulkanInstance = vulkan.into();

        let chain = librashader::runtime::vk::capi::FilterChainVulkan::load_from_preset(vulkan, *preset, options)?;

        unsafe {
            out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(
                chain,
            )))))
        }
    }
}

extern_fn! {
    /// Records rendering commands for a frame with the given parameters for the given filter chain
    /// to the input command buffer.
    ///
    /// librashader will not do any queue submissions.
    ///
    /// ## Safety
    /// - `libra_vk_filter_chain_frame` **must not be called within a RenderPass**.
    /// - `command_buffer` must be a valid handle to a `VkCommandBuffer` that is ready for recording.
    /// - `chain` may be null, invalid, but not uninitialized. If `chain` is null or invalid, this
    ///    function will return an error.
    /// - `mvp` may be null, or if it is not null, must be an aligned pointer to 16 consecutive `float`
    ///    values for the model view projection matrix.
    /// - `opt` may be null, or if it is not null, must be an aligned pointer to a valid `frame_vk_opt_t`
    ///    struct.
    fn libra_vk_filter_chain_frame(
        chain: *mut libra_vk_filter_chain_t,
        command_buffer: vk::CommandBuffer,
        frame_count: usize,
        image: libra_image_vk_t,
        viewport: libra_viewport_t,
        out: libra_image_vk_t,
        mvp: *const f32,
        opt: *const FrameOptionsVulkan
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        let image: VulkanImage = image.into();
        let output = out.into();
        let mvp = if mvp.is_null() {
            None
        } else {
            Some(<&[f32; 16]>::try_from(unsafe { slice::from_raw_parts(mvp, 16) }).unwrap())
        };
        let opt = if opt.is_null() {
            None
        } else {
            Some(unsafe { opt.read() })
        };
        let viewport = Viewport {
            x: viewport.x,
            y: viewport.y,
            output,
            mvp,
        };
        chain.frame(&image, &viewport, command_buffer, frame_count, opt.as_ref())?;
    }
}

extern_fn! {
    /// Sets a parameter for the filter chain.
    ///
    /// If the parameter does not exist, returns an error.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_vk_filter_chain_t`.
    /// - `param_name` must be either null or a null terminated string.
    fn libra_vk_filter_chain_set_param(
        chain: *mut libra_vk_filter_chain_t,
        param_name: *const c_char,
        value: f32
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        assert_non_null!(param_name);
        unsafe {
            let name = CStr::from_ptr(param_name);
            let name = name.to_str()?;

            if chain.set_parameter(name, value).is_none() {
                return LibrashaderError::UnknownShaderParameter(param_name).export()
            }
        }
    }
}

extern_fn! {
    /// Gets a parameter for the filter chain.
    ///
    /// If the parameter does not exist, returns an error.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_vk_filter_chain_t`.
    /// - `param_name` must be either null or a null terminated string.
    fn libra_vk_filter_chain_get_param(
        chain: *mut libra_vk_filter_chain_t,
        param_name: *const c_char,
        out: *mut MaybeUninit<f32>
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        assert_non_null!(param_name);
        unsafe {
            let name = CStr::from_ptr(param_name);
            let name = name.to_str()?;

            let Some(value) = chain.get_parameter(name) else {
                return LibrashaderError::UnknownShaderParameter(param_name).export()
            };

            out.write(MaybeUninit::new(value));
        }
    }
}

extern_fn! {
    /// Sets the number of active passes for this chain.
    ///
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_vk_filter_chain_t`.
    fn libra_vk_filter_chain_set_active_pass_count(
        chain: *mut libra_vk_filter_chain_t,
        value: u32
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        chain.set_enabled_pass_count(value as usize);
    }
}

extern_fn! {
    /// Gets the number of active passes for this chain.
    ///
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_vk_filter_chain_t`.
    fn libra_vk_filter_chain_get_active_pass_count(
        chain: *mut libra_vk_filter_chain_t,
        out: *mut MaybeUninit<u32>
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        let value = chain.get_enabled_pass_count();
        unsafe {
            out.write(MaybeUninit::new(value as u32))
        }
    }
}

extern_fn! {
    /// Free a Vulkan filter chain.
    ///
    /// The resulting value in `chain` then becomes null.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_vk_filter_chain_t`.
    fn libra_vk_filter_chain_free(
        chain: *mut libra_vk_filter_chain_t
    ) {
        assert_non_null!(chain);
        unsafe {
            let chain_ptr = &mut *chain;
            let chain = chain_ptr.take();
            drop(Box::from_raw(chain.unwrap().as_ptr()))
        };
    }
}
