#![feature(strict_provenance)]
#![feature(type_alias_impl_trait)]

mod binding;
mod filter_chain;
mod filter_pass;
mod framebuffer;
mod render_target;
mod util;

mod samplers;
mod texture;
pub mod options;
mod gl;


pub mod error;
pub use filter_chain::FilterChain;
pub use framebuffer::Viewport;

pub mod gl3 {
    pub use super::framebuffer::GLImage;
    pub type FilterChain = super::filter_chain::FilterChain<super::gl::gl3::CompatibilityGL>;
    pub type Viewport<'a> = super::framebuffer::Viewport<'a, <super::gl::gl3::CompatibilityGL as super::gl::GLInterface>::Framebuffer>;
}

pub mod gl46 {
    pub use super::framebuffer::GLImage;
    pub type FilterChain = super::filter_chain::FilterChain<super::gl::gl46::DirectStateAccessGL>;
    pub type Viewport<'a> = super::framebuffer::Viewport<'a, <super::gl::gl46::DirectStateAccessGL as super::gl::GLInterface>::Framebuffer>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter_chain::FilterChain;

    #[test]
    fn triangle_gl() {
        let (glfw, window, events, shader, vao) = gl::gl3::hello_triangle::setup();
        let mut filter =
           FilterChain::load_from_path("../test/slang-shaders/vhs/VHSPro.slangp", None)
            // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
                .unwrap();
        gl::gl3::hello_triangle::do_loop(glfw, window, events, shader, vao, &mut filter);
    }

    #[test]
    fn triangle_gl46() {
        let (glfw, window, events, shader, vao) = gl::gl46::hello_triangle::setup();
        let mut filter =
            FilterChain::load_from_path("../test/slang-shaders/vhs/VHSPro.slangp", None)
                // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
                .unwrap();
        gl::gl46::hello_triangle::do_loop(glfw, window, events, shader, vao, &mut filter);
    }
}
