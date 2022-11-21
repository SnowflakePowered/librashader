#![feature(strict_provenance)]
#![feature(type_alias_impl_trait)]

mod binding;
mod filter_chain;
mod filter_pass;
mod framebuffer;
mod hello_triangle;
mod render_target;
mod util;
mod quad_render;

pub use filter_chain::FilterChain;
pub use framebuffer::Viewport;
pub use framebuffer::GlImage;
pub use framebuffer::Size;
pub use framebuffer::Framebuffer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter_chain::FilterChain;

    #[test]
    fn triangle() {
        let (glfw, window, events, shader, vao) = hello_triangle::setup();
        let mut filter = FilterChain::load_from_path("../test/slang-shaders/crt/crt-royale-fake-bloom.slangp").unwrap();

        // FilterChain::load("../test/slang-shaders/crt/crt-royale.slangp").unwrap();

        hello_triangle::do_loop(glfw, window, events, shader, vao, &mut filter);
    }

    // #[test]
    // fn load_preset() {
    //
    //     load("../test/basic.slangp")
    //         .unwrap();
    // }
}
