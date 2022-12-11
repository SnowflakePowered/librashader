#![feature(type_alias_impl_trait)]
#![feature(let_chains)]

mod hello_triangle;
mod filter_chain;
mod filter_pass;
mod error;
mod util;
mod framebuffer;
mod vulkan_state;
mod draw_quad;
mod renderpass;
mod vulkan_primitives;

#[cfg(test)]
mod tests {
    use crate::filter_chain::FilterChainVulkan;
    use super::*;
    #[test]
    fn triangle_vk() {
        let base = hello_triangle::ExampleBase::new(900, 600);
        let mut filter = FilterChainVulkan::load_from_path(
            base.device.clone(),
            "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
            None
        )
            // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
            .unwrap();
        hello_triangle::main(base);
    }
}
