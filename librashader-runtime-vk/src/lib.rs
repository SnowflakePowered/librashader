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
mod ubo_ring;
mod util;
mod vulkan_primitives;
mod vulkan_state;

pub use filter_chain::FilterChain;
pub use filter_chain::VulkanDevice;
pub use filter_chain::VulkanInstance;
pub use texture::VulkanImage;

pub mod error;
pub mod options;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter_chain::FilterChain;
    use crate::hello_triangle::vulkan_base::VulkanBase;

    #[test]
    fn triangle_vk() {
        let entry = unsafe { ash::Entry::load().unwrap() };
        let base = VulkanBase::new(entry).unwrap();
        dbg!("finished");
        let mut filter = FilterChain::load_from_path(
            &base,
            // "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
            "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp",
            // "../test/basic.slangp",
            None,
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
