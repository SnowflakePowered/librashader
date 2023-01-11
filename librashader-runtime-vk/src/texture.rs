use crate::{error, util};
use crate::filter_chain::Vulkan;
use crate::util::find_vulkan_memory_type;
use crate::vulkan_primitives::VulkanImageMemory;
use ash::vk;
use ash::vk::{
    ImageAspectFlags, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, ImageViewType,
    SampleCountFlags, SharingMode,
};
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::Scale2D;
use librashader_runtime::scaling::{MipmapSize, ViewportSize};

pub struct OwnedTexture {
    pub device: ash::Device,
    pub mem_props: vk::PhysicalDeviceMemoryProperties,
    pub image_view: vk::ImageView,
    pub image: VulkanImage,
    pub memory: VulkanImageMemory,
    pub max_miplevels: u32,
}

impl OwnedTexture {
    fn new_internal(
        device: ash::Device,
        mem_props: vk::PhysicalDeviceMemoryProperties,
        size: Size<u32>,
        format: ImageFormat,
        max_miplevels: u32,
    ) -> error::Result<OwnedTexture> {
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(ImageType::TYPE_2D)
            .format(format.into())
            .extent(size.into())
            .mip_levels(std::cmp::min(max_miplevels, size.calculate_miplevels()))
            .array_layers(1)
            .samples(SampleCountFlags::TYPE_1)
            .tiling(ImageTiling::OPTIMAL)
            .flags(vk::ImageCreateFlags::MUTABLE_FORMAT)
            .usage(
                ImageUsageFlags::SAMPLED
                    | ImageUsageFlags::COLOR_ATTACHMENT
                    | ImageUsageFlags::TRANSFER_DST
                    | ImageUsageFlags::TRANSFER_SRC,
            )
            .sharing_mode(SharingMode::EXCLUSIVE)
            .initial_layout(ImageLayout::UNDEFINED)
            .build();

        let image = unsafe { device.create_image(&image_create_info, None)? };
        let mem_reqs = unsafe { device.get_image_memory_requirements(image.clone()) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_reqs.size)
            .memory_type_index(find_vulkan_memory_type(
                &mem_props,
                mem_reqs.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ))
            .build();

        // todo: optimize by reusing existing memory.
        let memory = VulkanImageMemory::new(&device, &alloc_info)?;
        memory.bind(&image)?;

        let image_subresource = vk::ImageSubresourceRange::builder()
            .base_mip_level(0)
            .base_array_layer(0)
            .level_count(image_create_info.mip_levels)
            .layer_count(1)
            .aspect_mask(ImageAspectFlags::COLOR)
            .build();

        let swizzle_components = vk::ComponentMapping::builder()
            .r(vk::ComponentSwizzle::R)
            .g(vk::ComponentSwizzle::G)
            .b(vk::ComponentSwizzle::B)
            .a(vk::ComponentSwizzle::A)
            .build();

        let mut view_info = vk::ImageViewCreateInfo::builder()
            .view_type(ImageViewType::TYPE_2D)
            .format(format.into())
            .image(image.clone())
            .subresource_range(image_subresource)
            .components(swizzle_components)
            .build();

        let image_view = unsafe { device.create_image_view(&view_info, None)? };

        Ok(OwnedTexture {
            device,
            mem_props,
            image_view,
            image: VulkanImage {
                image,
                size,
                format: format.into(),
            },
            memory,
            max_miplevels,
        })
    }

    pub fn new(
        vulkan: &Vulkan,
        size: Size<u32>,
        format: ImageFormat,
        max_miplevels: u32,
    ) -> error::Result<OwnedTexture> {
        Self::new_internal(vulkan.device.clone(), vulkan.memory_properties, size, format, max_miplevels)
    }

    pub(crate) fn scale(
        &mut self,
        scaling: Scale2D,
        format: ImageFormat,
        viewport_size: &Size<u32>,
        _original: &InputTexture,
        source: &InputTexture,
    ) -> error::Result<Size<u32>> {


        let size = source.image.size.scale_viewport(scaling, *viewport_size);

        if self.image.size != size {
            let mut new = OwnedTexture::new_internal(self.device.clone(), self.mem_props, size, if format == ImageFormat::Unknown {
                ImageFormat::R8G8B8A8Unorm
            } else {
                format
            }, self.max_miplevels)?;

            let old = std::mem::replace(self, new);
            drop(old)
        }
        Ok(size)
    }


    pub fn create_image_view(&self) -> error::Result<vk::ImageView> {
        let image_subresource = vk::ImageSubresourceRange::builder()
            .base_mip_level(0)
            .base_array_layer(0)
            .level_count(1)
            .layer_count(1)
            .aspect_mask(ImageAspectFlags::COLOR)
            .build();

        let swizzle_components = vk::ComponentMapping::builder()
            .r(vk::ComponentSwizzle::R)
            .g(vk::ComponentSwizzle::G)
            .b(vk::ComponentSwizzle::B)
            .a(vk::ComponentSwizzle::A)
            .build();

        let mut view_info = vk::ImageViewCreateInfo::builder()
            .view_type(ImageViewType::TYPE_2D)
            .format(self.image.format)
            .image(self.image.image.clone())
            .subresource_range(image_subresource)
            .components(swizzle_components)
            .build();

        let image_view = unsafe { self.device.create_image_view(&view_info, None)? };
        Ok(image_view)
    }

    pub fn as_input(&self, filter: FilterMode, wrap_mode: WrapMode) -> error::Result<InputTexture> {
        Ok(InputTexture {
            image: self.image.clone(),
            image_view: self.create_image_view()?,
            wrap_mode,
            filter_mode: filter,
            mip_filter: filter,
        })
    }
}

impl Drop for OwnedTexture {
    fn drop(&mut self) {
        unsafe {
            if self.image_view != vk::ImageView::null() {
                self.device.destroy_image_view(self.image_view, None);
            }
            if self.image.image != vk::Image::null() {
                self.device.destroy_image(self.image.image, None);
            }
        }
    }
}

#[derive(Clone)]
pub struct VulkanImage {
    pub size: Size<u32>,
    pub image: vk::Image,
    pub format: vk::Format,
}


#[derive(Clone)]
pub struct InputTexture {
    pub image: VulkanImage,
    pub image_view: vk::ImageView,
    pub wrap_mode: WrapMode,
    pub filter_mode: FilterMode,
    pub mip_filter: FilterMode,
}
