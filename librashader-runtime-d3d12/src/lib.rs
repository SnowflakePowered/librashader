#![feature(const_trait_impl)]
#![feature(let_chains)]
#![feature(type_alias_impl_trait)]
mod buffer;
mod descriptor_heap;
mod error;
mod filter_chain;
mod filter_pass;
mod framebuffer;
mod graphics_pipeline;
mod hello_triangle;
mod luts;
mod mipmap;
mod quad_render;
mod render_target;
mod samplers;
mod texture;
mod util;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hello_triangle::{DXSample, SampleCommandLine};

    #[test]
    fn triangle_d3d12() {
        let sample = hello_triangle::d3d12_hello_triangle::Sample::new(
            // "../test/slang-shaders/crt/crt-lottes.slangp",
            "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp",
            &SampleCommandLine {
                use_warp_device: false,
            },
        )
        .unwrap();
        hello_triangle::main(sample).unwrap()
    }
}
