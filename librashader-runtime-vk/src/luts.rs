use ash::vk;
use ash::vk::ImageSubresourceLayers;
use glfw::Key::P;
use librashader_common::{FilterMode, WrapMode};
use librashader_presets::TextureConfig;
use librashader_runtime::image::{BGRA8, Image};
use librashader_runtime::scaling::MipmapSize;
use crate::filter_chain::Vulkan;
use crate::{error, util};
use crate::vulkan_primitives::{VulkanBuffer, VulkanImageMemory};

pub struct LutTexture {
    pub texture: vk::Image,
    pub texture_view: vk::ImageView,
    pub memory: VulkanImageMemory,
    pub staging: VulkanBuffer,
    pub filter_mode: FilterMode,
    pub wrap_mode: WrapMode,
    pub mipmap: bool,
}

impl LutTexture {
    pub fn new(vulkan: &Vulkan, cmd: &vk::CommandBuffer, image: Image<BGRA8>, config: &TextureConfig) -> error::Result<LutTexture> {

        // todo: might need to use bgra8
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::B8G8R8A8_UNORM)
            .extent(image.size.into())
            .mip_levels(if config.mipmap {
                image.size.calculate_miplevels()
            } else {
                1
            })
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .build();

        let texture = unsafe {
           vulkan.device.create_image(&image_info, None)?
        };

        let memory = unsafe {
            let mem_reqs = vulkan.device.get_image_memory_requirements(texture.clone());
            let mem_type = util::find_vulkan_memory_type(&vulkan.memory_properties, mem_reqs.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL);
            crate::vulkan_primitives::VulkanImageMemory::new(&vulkan.device, &vk::MemoryAllocateInfo::builder()
                .memory_type_index(mem_type)
                .allocation_size(mem_reqs.size))?
        };

        memory.bind(&texture)?;

        let image_subresource = vk::ImageSubresourceRange::builder()
            .level_count(image_info.mip_levels)
            .layer_count(1)
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .build();

        let swizzle_components = vk::ComponentMapping::builder()
            .r(vk::ComponentSwizzle::R)
            .g(vk::ComponentSwizzle::G)
            .b(vk::ComponentSwizzle::B)
            .a(vk::ComponentSwizzle::A)
            .build();

        let mut view_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::B8G8R8A8_UNORM)
            .image(texture.clone())
            .subresource_range(image_subresource)
            .components(swizzle_components)
            .build();

        let texture_view = unsafe {
            vulkan.device.create_image_view(&view_info, None)?
        };

        let mut staging = VulkanBuffer::new(&vulkan.device, &vulkan.memory_properties, vk::BufferUsageFlags::TRANSFER_SRC, image.bytes.len())?;
        unsafe {
            let mut handle = staging.map()?;
            handle.copy_from(&image.bytes)
        }

        unsafe {
            util::vulkan_image_layout_transition_levels(&vulkan.device, *cmd, texture,
                                                        vk::REMAINING_MIP_LEVELS,
                                                        vk::ImageLayout::UNDEFINED,
                if config.mipmap {
                    vk::ImageLayout::GENERAL
                } else {
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL
                },
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                    vk::QUEUE_FAMILY_IGNORED,
                vk::QUEUE_FAMILY_IGNORED
            );

            vulkan.device.cmd_copy_buffer_to_image(*cmd,
                                                   staging.handle,
                                                   texture,
                                                   if config.mipmap { vk::ImageLayout::GENERAL } else { vk::ImageLayout::TRANSFER_DST_OPTIMAL },
                                                   &[vk::BufferImageCopy::builder()
                                                       .image_subresource(vk::ImageSubresourceLayers::builder()
                                                           .aspect_mask(vk::ImageAspectFlags::COLOR)
                                                           .mip_level(0)
                                                           .base_array_layer(0)
                                                           .layer_count(1)
                                                           .build())
                                                       .image_extent(image.size.into()).build()])

        }

        // generate mipmaps
        for level in 1..image_info.mip_levels {
            let source_size = image.size.scale_mipmap(level - 1);
            let target_size = image.size.scale_mipmap(level);

            let src_offsets = [
                vk::Offset3D {
                    x: 0,
                    y: 0,
                    z: 0,
                },
                vk::Offset3D {
                    x: source_size.width as i32,
                    y: source_size.height as i32,
                    z: 1,
                },
            ];

            let dst_offsets = [
                vk::Offset3D {
                    x: 0,
                    y: 0,
                    z: 0,
                },
                vk::Offset3D {
                    x: target_size.width as i32,
                    y: target_size.height as i32,
                    z: 1,
                },
            ];
            let src_subresource = ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(level - 1)
                .base_array_layer(0)
                .layer_count(1)
                .build();

            let dst_subresource = ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(level)
                .base_array_layer(0)
                .layer_count(1)
                .build();

            let image_blit = [vk::ImageBlit::builder()
                .src_subresource(src_subresource)
                .src_offsets(src_offsets)
                .dst_subresource(dst_subresource)
                .dst_offsets(dst_offsets)
                .build()];

            unsafe {
                util::vulkan_image_layout_transition_levels(&vulkan.device, *cmd, texture,
                                                            vk::REMAINING_MIP_LEVELS,
                                                            vk::ImageLayout::GENERAL,
                                                            vk::ImageLayout::GENERAL,
                                                            vk::AccessFlags::TRANSFER_WRITE,
                                                            vk::AccessFlags::TRANSFER_READ,
                                                            vk::PipelineStageFlags::TRANSFER,
                                                            vk::PipelineStageFlags::TRANSFER,
                                                            vk::QUEUE_FAMILY_IGNORED,
                                                            vk::QUEUE_FAMILY_IGNORED
                );

                // todo: respect mipmap filter?
                vulkan.device.cmd_blit_image(*cmd, texture, vk::ImageLayout::GENERAL, texture, vk::ImageLayout::GENERAL, &image_blit, vk::Filter::LINEAR);
            }
        }

        unsafe {
            util::vulkan_image_layout_transition_levels(&vulkan.device, *cmd, texture,
                                                        vk::REMAINING_MIP_LEVELS,
                                                        if config.mipmap { vk::ImageLayout::GENERAL } else { vk::ImageLayout::TRANSFER_DST_OPTIMAL },
                                                        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                                                        vk::AccessFlags::TRANSFER_WRITE,
                                                        vk::AccessFlags::TRANSFER_READ,
                                                        vk::PipelineStageFlags::TRANSFER,
                                                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                                                        vk::QUEUE_FAMILY_IGNORED,
                                                        vk::QUEUE_FAMILY_IGNORED
            );
        }

        Ok(LutTexture {
            texture,
            texture_view,
            memory,
            staging,
            filter_mode: config.filter_mode,
            wrap_mode: config.wrap_mode,
            mipmap: config.mipmap
        })
    }
}