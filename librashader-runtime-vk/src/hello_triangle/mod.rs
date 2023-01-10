mod command;
mod debug;
mod framebuffer;
mod physicaldevice;
mod pipeline;
mod surface;
mod swapchain;
mod syncobjects;
pub mod vulkan_base;

use crate::filter_chain::{FilterChainVulkan, Vulkan};
use crate::hello_triangle::command::VulkanCommandPool;
use crate::hello_triangle::framebuffer::VulkanFramebuffer;
use crate::hello_triangle::pipeline::VulkanPipeline;
use crate::hello_triangle::surface::VulkanSurface;
use crate::hello_triangle::swapchain::VulkanSwapchain;
use crate::hello_triangle::syncobjects::SyncObjects;
use crate::hello_triangle::vulkan_base::VulkanBase;
use crate::util;
use ash::vk;
use ash::vk::RenderingInfo;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::platform::windows::EventLoopBuilderExtWindows;
use crate::texture::VulkanImage;

// Constants
const WINDOW_TITLE: &'static str = "librashader Vulkan";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

struct VulkanWindow;

impl VulkanWindow {
    fn init_window(event_loop: &EventLoop<()>) -> winit::window::Window {
        winit::window::WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_resizable(false)
            .build(event_loop)
            .expect("Failed to create window.")
    }

    pub fn main_loop(
        event_loop: EventLoop<()>,
        window: winit::window::Window,
        vulkan: VulkanDraw,
        mut filter_chain: FilterChainVulkan,
    ) {
        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        virtual_keycode,
                        state,
                        ..
                    } => match (virtual_keycode, state) {
                        (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                            *control_flow = ControlFlow::Exit
                        }
                        _ => {}
                    },
                },
                _ => {}
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_window_id) => {
                VulkanWindow::draw_frame(&vulkan, &mut filter_chain);
            }
            _ => (),
        })
    }

    unsafe fn record_command_buffer(
        vulkan: &VulkanDraw,
        framebuffer: vk::Framebuffer,
        cmd: vk::CommandBuffer,
    ) {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.3, 0.3, 0.5, 0.0],
            },
        }];

        let render_pass_begin = vk::RenderPassBeginInfo::builder()
            .render_pass(vulkan.pipeline.renderpass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                extent: vulkan.swapchain.extent,
                ..Default::default()
            })
            .clear_values(&clear_values)
            .build();

        vulkan
            .base
            .device
            .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
            .expect("could not reset command buffer");

        vulkan
            .base
            .device
            .begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default())
            .expect("failed to begin command buffer");

        vulkan.base.device.cmd_begin_render_pass(
            cmd,
            &render_pass_begin,
            vk::SubpassContents::INLINE,
        );

        vulkan.base.device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            vulkan.pipeline.graphic_pipeline,
        );

        vulkan.base.device.cmd_set_viewport(
            cmd,
            0,
            &[vk::Viewport {
                max_depth: 1.0,
                width: vulkan.swapchain.extent.width as f32,
                height: vulkan.swapchain.extent.height as f32,
                ..Default::default()
            }],
        );

        vulkan.base.device.cmd_set_scissor(
            cmd,
            0,
            &[vk::Rect2D {
                offset: Default::default(),
                extent: vulkan.swapchain.extent,
            }],
        );

        vulkan.base.device.cmd_draw(cmd, 3, 1, 0, 0);
        vulkan.base.device.cmd_end_render_pass(cmd);
    }

    fn draw_frame(vulkan: &VulkanDraw, filter: &mut FilterChainVulkan) {
        unsafe {
            vulkan
                .base
                .device
                .wait_for_fences(&[vulkan.sync.in_flight], true, u64::MAX)
                .unwrap();
            vulkan
                .base
                .device
                .reset_fences(&[vulkan.sync.in_flight])
                .unwrap();

            let (swapchain_index, _) = vulkan
                .swapchain
                .loader
                .acquire_next_image(
                    vulkan.swapchain.swapchain,
                    u64::MAX,
                    vulkan.sync.image_available,
                    vk::Fence::null(),
                )
                .unwrap();

            let cmd = vulkan.render_command_pool.buffers[swapchain_index as usize];
            let framebuffer = vulkan.render_framebuffers[swapchain_index as usize].framebuffer;
            let framebuffer_image = vulkan.swapchain.render_images[swapchain_index as usize].0;
            let swapchain_image = vulkan.swapchain.swapchain_images[swapchain_index as usize];

            Self::record_command_buffer(vulkan, framebuffer, cmd);

            filter.frame(0, &vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: vulkan.swapchain.extent.width as f32,
                height: vulkan.swapchain.extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }, &VulkanImage {
                size: vulkan.swapchain.extent.into(),
                image: framebuffer_image,
                format: vulkan.swapchain.format.format,
            }, cmd, None)
                .unwrap();

            util::vulkan_image_layout_transition_levels(
                &vulkan.base.device,
                cmd,
                framebuffer_image,
                1,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::AccessFlags::TRANSFER_READ,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::TRANSFER,
                vk::QUEUE_FAMILY_IGNORED,
                vk::QUEUE_FAMILY_IGNORED,
            );

            let blit_subresource = vk::ImageSubresourceLayers::builder()
                .layer_count(1)
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .build();

            let src_offsets = [
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: vulkan.swapchain.extent.width as i32,
                    y: vulkan.swapchain.extent.height as i32,
                    z: 1,
                },
            ];

            let dst_offsets = [
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: vulkan.swapchain.extent.width as i32,
                    y: vulkan.swapchain.extent.height as i32,
                    z: 1,
                },
            ];
            vulkan.base.device.cmd_blit_image(
                cmd,
                framebuffer_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                swapchain_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::ImageBlit {
                    src_subresource: blit_subresource,
                    src_offsets,
                    dst_subresource: blit_subresource,
                    dst_offsets,
                }],
                vk::Filter::LINEAR,
            );

            util::vulkan_image_layout_transition_levels(
                &vulkan.base.device,
                cmd,
                swapchain_image,
                1,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::PRESENT_SRC_KHR,
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                vk::QUEUE_FAMILY_IGNORED,
                vk::QUEUE_FAMILY_IGNORED,
            );

            vulkan
                .base
                .device
                .end_command_buffer(cmd)
                .expect("failed to record commandbuffer");

            let submit_info = vk::SubmitInfo::builder()
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .wait_semaphores(&[vulkan.sync.image_available])
                .signal_semaphores(&[vulkan.sync.render_finished])
                .command_buffers(&[cmd])
                .build();

            vulkan
                .base
                .device
                .queue_submit(
                    vulkan.base.graphics_queue,
                    &[submit_info],
                    vulkan.sync.in_flight,
                )
                .expect("failed to submit queue");

            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&[vulkan.sync.render_finished])
                .swapchains(&[vulkan.swapchain.swapchain])
                .image_indices(&[swapchain_index])
                .build();

            vulkan
                .swapchain
                .loader
                .queue_present(vulkan.base.graphics_queue, &present_info)
                .unwrap();
        }
    }
}

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

pub struct VulkanDraw {
    surface: VulkanSurface,
    base: VulkanBase,
    pub swapchain: VulkanSwapchain,
    pub pipeline: VulkanPipeline,
    pub swapchain_framebuffers: Vec<VulkanFramebuffer>,
    pub swapchain_command_pool: VulkanCommandPool,
    pub render_command_pool: VulkanCommandPool,
    pub render_framebuffers: Vec<VulkanFramebuffer>,
    pub sync: SyncObjects,
}

pub fn main(vulkan: VulkanBase, filter_chain: FilterChainVulkan) {
    let event_loop = EventLoopBuilder::new()
        .with_any_thread(true)
        .with_dpi_aware(true)
        .build();

    let window = VulkanWindow::init_window(&event_loop);
    let surface = VulkanSurface::new(&vulkan, &window).unwrap();

    let swapchain = VulkanSwapchain::new(&vulkan, &surface, WINDOW_WIDTH, WINDOW_HEIGHT).unwrap();

    let pipeline = unsafe { VulkanPipeline::new(&vulkan, &swapchain) }.unwrap();

    let mut swapchain_framebuffers = vec![];
    for image in &swapchain.swapchain_image_views {
        swapchain_framebuffers.push(
            VulkanFramebuffer::new(
                &vulkan.device,
                image,
                &pipeline.renderpass,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
            )
            .unwrap(),
        )
    }

    let mut render_framebuffers = vec![];
    for image in &swapchain.render_image_views {
        render_framebuffers.push(
            VulkanFramebuffer::new(
                &vulkan.device,
                image,
                &pipeline.renderpass,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
            )
            .unwrap(),
        )
    }

    let swapchain_command_pool = VulkanCommandPool::new(&vulkan, 3).unwrap();
    let render_command_pool = VulkanCommandPool::new(&vulkan, 3).unwrap();

    let sync = SyncObjects::new(&vulkan.device).unwrap();

    let vulkan = VulkanDraw {
        surface,
        swapchain,
        base: vulkan,
        pipeline,
        swapchain_framebuffers,
        swapchain_command_pool,
        sync,
        render_command_pool,
        render_framebuffers,
    };

    VulkanWindow::main_loop(event_loop, window, vulkan, filter_chain);
}
