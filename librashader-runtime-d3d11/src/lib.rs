#![feature(type_alias_impl_trait)]
#![feature(let_chains)]

mod filter_chain;

mod filter_pass;
mod framebuffer;
#[cfg(test)]
mod hello_triangle;
mod quad_render;
mod render_target;
mod samplers;
mod texture;
mod util;

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn triangle_d3d11() {
        // let sample = hello_triangle::d3d11_hello_triangle::Sample::new("../test/slang-shaders/crt/crt-royale.slangp").unwrap();
        let sample = hello_triangle::d3d11_hello_triangle::Sample::new(
            "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp",
        )
        .unwrap();

        // let sample = hello_triangle::d3d11_hello_triangle::Sample::new("../test/basic.slangp").unwrap();

        hello_triangle::main(sample).unwrap();
    }
}
