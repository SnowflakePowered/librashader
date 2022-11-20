#![feature(strict_provenance)]

mod hello_triangle;
mod filter_pass;
mod util;
mod framebuffer;
mod binding;
mod filter_chain;
mod render_target;


#[cfg(test)]
mod tests {
    use crate::filter_chain::FilterChain;
    use super::*;

    #[test]
    fn triangle() {
        let (glfw, window, events, shader, vao) = hello_triangle::setup();
        let mut filter = FilterChain::load("../test/basic.slangp").unwrap();


        // FilterChain::load("../test/slang-shaders/crt/crt-royale.slangp").unwrap();

        hello_triangle::do_loop(glfw, window, events, shader, vao, &mut filter );
    }

    // #[test]
    // fn load_preset() {
    //
    //     load("../test/basic.slangp")
    //         .unwrap();
    // }
}
