#![feature(strict_provenance)]
#![feature(type_alias_impl_trait)]

mod binding;
mod filter_chain;
mod filter_pass;
mod framebuffer;
mod quad_render;
mod render_target;
mod util;
pub mod error;

mod samplers;

pub use filter_chain::FilterChain;
pub use framebuffer::Framebuffer;
pub use framebuffer::GlImage;
pub use framebuffer::Viewport;

#[cfg(test)]
mod hello_triangle;
mod texture;
mod options;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter_chain::FilterChain;

    #[test]
    fn triangle_gl() {
        let (glfw, window, events, shader, vao) = hello_triangle::setup();
        let mut filter =
//            FilterChain::load_from_path("../test/slang-shaders/vhs/VHSPro.slangp", None)
            FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
                .unwrap();
        hello_triangle::do_loop(glfw, window, events, shader, vao, &mut filter);
    }
}
