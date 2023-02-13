#![cfg(target_os = "windows")]
#![feature(const_trait_impl)]
#![feature(let_chains)]
#![feature(type_alias_impl_trait)]
#![feature(int_roundings)]

mod buffer;
mod descriptor_heap;
pub mod error;
mod filter_chain;
mod filter_pass;
mod framebuffer;
mod graphics_pipeline;
mod hello_triangle;
mod luts;
mod mipmap;
pub mod options;
mod parameters;
mod quad_render;
mod samplers;
mod texture;
mod util;

pub use filter_chain::FilterChainD3D12;
pub use texture::D3D12InputImage;
pub use texture::D3D12OutputView;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hello_triangle::{DXSample, SampleCommandLine};

    #[test]
    fn triangle_d3d12() {
        let sample = hello_triangle::d3d12_hello_triangle::Sample::new(
            // "../test/slang-shaders/crt/crt-lottes.slangp",
            // "../test/basic.slangp",
            //     "../test/slang-shaders/handheld/console-border/gbc-lcd-grid-v2.slangp",
            "../test/Mega_Bezel_Packs/Duimon-Mega-Bezel/Presets/Advanced/Nintendo_GBA_SP/GBA_SP-[ADV]-[LCD-GRID]-[Night].slangp",
            // "../test/slang-shaders/test/feedback.slangp",
            // "../test/slang-shaders/test/history.slangp",
            // "../test/slang-shaders/crt/crt-royale.slangp",
            // "../test/slang-shaders/vhs/VHSPro.slangp",
            &SampleCommandLine {
                use_warp_device: false,
            },
        )
        .unwrap();
        hello_triangle::main(sample).unwrap()
    }
}
