#![feature(const_trait_impl)]
#![feature(let_chains)]

mod error;
mod filter_chain;
mod heap;
mod hello_triangle;
mod samplers;
mod texture;
mod util;
mod mipmap;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hello_triangle::{DXSample, SampleCommandLine};

    #[test]
    fn triangle_d3d12() {
        let sample = hello_triangle::d3d12_hello_triangle::Sample::new(
            "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
            &SampleCommandLine {
                use_warp_device: false,
            },
        )
        .unwrap();
        hello_triangle::main(sample).unwrap()
    }
}
