pub mod vulkan_base;
mod debug;
mod physicaldevice;
mod surface;

use winit::event::{Event, VirtualKeyCode, ElementState, KeyboardInput, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow, EventLoopBuilder};
use winit::platform::windows::EventLoopBuilderExtWindows;
use crate::filter_chain::FilterChainVulkan;
use crate::hello_triangle::surface::VulkanSurface;
use crate::hello_triangle::vulkan_base::VulkanBase;

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
            .build(event_loop)
            .expect("Failed to create window.")
    }

    pub fn main_loop(event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            match event {
                | Event::WindowEvent { event, .. } => {
                    match event {
                        | WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit
                        },
                        | WindowEvent::KeyboardInput { input, .. } => {
                            match input {
                                | KeyboardInput { virtual_keycode, state, .. } => {
                                    match (virtual_keycode, state) {
                                        | (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                                            dbg!();
                                            *control_flow = ControlFlow::Exit
                                        },
                                        | _ => {},
                                    }
                                },
                            }
                        },
                        | _ => {},
                    }
                },
                _ => (),
            }

        })
    }
}

pub struct VulkanDraw {
    surface: VulkanSurface,
    base: VulkanBase,
}

pub fn main(vulkan: VulkanBase, filter_chain: FilterChainVulkan) {
    let event_loop = EventLoopBuilder::new()
        .with_any_thread(true)
        .with_dpi_aware(true)
        .build();

    let window = VulkanWindow::init_window(&event_loop);
    let surface = VulkanSurface::new(&vulkan, &window)
        .unwrap();

    let vulkan = VulkanDraw {
        surface,
        base: vulkan
    };

    VulkanWindow::main_loop(event_loop);
}