#![feature(strict_provenance)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]

mod binding;
mod filter_chain;
mod filter_pass;
mod framebuffer;
mod render_target;
mod util;

mod gl;
mod samplers;
mod texture;

pub mod options;
pub mod error;
pub use filter_chain::FilterChain;
pub use framebuffer::Viewport;
pub use framebuffer::GLImage;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter_chain::FilterChain;
    use crate::options::FilterChainOptions;

    #[test]
    fn triangle_gl() {
        let (glfw, window, events, shader, vao) = gl::gl3::hello_triangle::setup();
        let mut filter =
            FilterChain::load_from_path("../test/slang-shaders/vhs/VHSPro.slangp", Some(&FilterChainOptions {
                gl_version: 0,
                use_dsa: false,
            }))
                // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
                .unwrap();
        gl::gl3::hello_triangle::do_loop(glfw, window, events, shader, vao, &mut filter);
    }

    #[test]
    fn triangle_gl46() {
        let (glfw, window, events, shader, vao) = gl::gl46::hello_triangle::setup();
        let mut filter =
            FilterChain::load_from_path("../test/slang-shaders/vhs/VHSPro.slangp", Some(&FilterChainOptions {
                gl_version: 0,
                use_dsa: true,
            }))
                // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
                .unwrap();
        gl::gl46::hello_triangle::do_loop(glfw, window, events, shader, vao, &mut filter);
    }
}
