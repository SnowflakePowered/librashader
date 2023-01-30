//! librashader Vulkan runtime
//!
//! This crate should not be used directly.
//! See [`librashader::runtime::vk`](https://docs.rs/librashader/latest/librashader/runtime/vk/index.html) instead.

#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(strict_provenance)]

mod draw_quad;
mod filter_chain;
mod filter_pass;
mod framebuffer;
#[cfg(test)]
mod hello_triangle;
mod luts;
mod parameters;
mod queue_selection;
mod render_target;
mod samplers;
mod texture;
mod util;
mod vulkan_primitives;
mod vulkan_state;

pub use filter_chain::FilterChainVulkan;
pub use filter_chain::VulkanInstance;
pub use filter_chain::VulkanObjects;
pub use texture::VulkanImage;

pub mod error;
pub mod options;
mod render_pass;

#[cfg(test)]
mod tests {
    use crate::filter_chain::FilterChainVulkan;
    use crate::hello_triangle::vulkan_base::VulkanBase;
    use crate::options::FilterChainOptionsVulkan;
    use ash::vk;

    #[test]
    fn triangle_vk() {
        let entry = unsafe { ash::Entry::load().unwrap() };
        let base = VulkanBase::new(entry).unwrap();
        dbg!("finished");
        let filter = FilterChainVulkan::load_from_path(
            &base,
            // "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
            "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__2__ADV-NO-REFLECT.slangp",
            // "../test/basic.slangp",
            Some(&FilterChainOptionsVulkan {
                frames_in_flight: 3,
                force_no_mipmaps: false,
                use_render_pass: true,
            }),
        )
        .unwrap();

        crate::hello_triangle::main(base, filter)

        // let base = hello_triangle_old::ExampleBase::new(900, 600);
        // // let mut filter = FilterChainVulkan::load_from_path(
        // //     (base.device.clone(), base.present_queue.clone(), base.device_memory_properties.clone()),
        // //     "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
        // //     None
        // // )
        //
        // let mut filter = FilterChainVulkan::load_from_path(
        //     (
        //         base.device.clone(),
        //         base.present_queue.clone(),
        //         base.device_memory_properties.clone(),
        //     ),
        //     "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
        //     None,
        // )
        // // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
        // .unwrap();
        // hello_triangle_old::main(base, filter);
    }
}
