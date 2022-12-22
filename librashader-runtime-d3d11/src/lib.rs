#![feature(type_alias_impl_trait)]
#![feature(let_chains)]

#[cfg(test)]
mod hello_triangle;

pub mod error;
mod filter_chain;
mod filter_pass;
mod framebuffer;
pub mod options;
mod parameters;
mod quad_render;
mod render_target;
mod samplers;
mod texture;
mod util;
mod viewport;

pub use filter_chain::FilterChainD3D11;
pub use texture::DxImageView;
pub use viewport::Viewport;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_d3d11() {
        let sample = hello_triangle::d3d11_hello_triangle::Sample::new(
            "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
            None,
        )
        .unwrap();
        // let sample = hello_triangle::d3d11_hello_triangle::Sample::new(
        //     "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp",
        //     Some(&FilterChainOptions {
        //         use_deferred_context: true,
        //     })
        // )
        // .unwrap();

        // let sample = hello_triangle::d3d11_hello_triangle::Sample::new("../test/basic.slangp").unwrap();

        hello_triangle::main(sample).unwrap();
    }
}
