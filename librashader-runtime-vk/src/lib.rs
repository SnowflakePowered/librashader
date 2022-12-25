#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(strict_provenance)]

mod draw_quad;
mod error;
mod filter_chain;
mod filter_pass;
mod framebuffer;
mod hello_triangle;
mod luts;
mod renderpass;
mod util;
mod vulkan_primitives;
mod vulkan_state;
mod samplers;
mod texture;
mod rendertarget;
mod ubo_ring;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter_chain::FilterChainVulkan;
    #[test]
    fn triangle_vk() {
        let base = hello_triangle::ExampleBase::new(900, 600);
        // let mut filter = FilterChainVulkan::load_from_path(
        //     (base.device.clone(), base.present_queue.clone(), base.device_memory_properties.clone()),
        //     "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
        //     None
        // )

        let mut filter = FilterChainVulkan::load_from_path(
            (
                base.device.clone(),
                base.present_queue.clone(),
                base.device_memory_properties.clone(),
            ),
            "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
            None,
        )
        // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
        .unwrap();
        hello_triangle::main(base);
    }
}
