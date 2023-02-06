#![cfg(target_os = "windows")]
//! librashader Direct3D 11 runtime
//!
//! This crate should not be used directly.
//! See [`librashader::runtime::d3d11`](https://docs.rs/librashader/latest/librashader/runtime/d3d11/index.html) instead.

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
mod draw_quad;
mod render_target;
mod samplers;
mod texture;
mod util;

pub use filter_chain::FilterChainD3D11;
pub use texture::D3D11InputView;
pub use texture::D3D11OutputView;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::FilterChainOptionsD3D11;
    use librashader_runtime::image::{Image, UVDirection};
    use std::env;

    // "../test/slang-shaders/scalefx/scalefx-9x.slangp",
    // "../test/slang-shaders/bezel/koko-aio/monitor-bloom.slangp",
    // "../test/slang-shaders/presets/crt-geom-ntsc-upscale-sharp.slangp",
    // "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp",
    // "../test/null.slangp",
    const FILTER_PATH: &str = "../test/slang-shaders/scalefx/scalefx-9x.slangp";

    // const FILTER_PATH: &str = "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp";
    const IMAGE_PATH: &str = "../test/finalfightlong.png";
    #[test]
    fn triangle_d3d11_args() {
        let mut args = env::args();
        let _ = args.next();
        let _ = args.next();
        let filter = args.next();
        let image = args
            .next()
            .and_then(|f| Image::load(f, UVDirection::TopLeft).ok())
            .or_else(|| Some(Image::load(IMAGE_PATH, UVDirection::TopLeft).unwrap()))
            .unwrap();

        let sample = hello_triangle::d3d11_hello_triangle::Sample::new(
            filter.as_deref().unwrap_or(FILTER_PATH),
            Some(&FilterChainOptionsD3D11 {
                use_deferred_context: false,
                force_no_mipmaps: false,
            }),
            // replace below with 'None' for the triangle
            Some(image),
        )
        .unwrap();
        // let sample = hello_triangle_old::d3d11_hello_triangle::Sample::new(
        //     "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp",
        //     Some(&FilterChainOptions {
        //         use_deferred_context: true,
        //     })
        // )
        // .unwrap();

        // let sample = hello_triangle_old::d3d11_hello_triangle::Sample::new("../test/basic.slangp").unwrap();

        hello_triangle::main(sample).unwrap();
    }

    #[test]
    fn triangle_d3d11() {
        let sample = hello_triangle::d3d11_hello_triangle::Sample::new(
            FILTER_PATH,
            Some(&FilterChainOptionsD3D11 {
                use_deferred_context: false,
                force_no_mipmaps: false,
            }),
            // replace below with 'None' for the triangle
            // None,
            Some(Image::load(IMAGE_PATH, UVDirection::TopLeft).unwrap())
        )
        .unwrap();
        // let sample = hello_triangle_old::d3d11_hello_triangle::Sample::new(
        //     "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp",
        //     Some(&FilterChainOptions {
        //         use_deferred_context: true,
        //     })
        // )
        // .unwrap();

        // let sample = hello_triangle_old::d3d11_hello_triangle::Sample::new("../test/basic.slangp").unwrap();

        hello_triangle::main(sample).unwrap();
    }
}
