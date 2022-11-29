use gl::types::GLuint;
use rustc_hash::FxHashMap;
use librashader_presets::TextureConfig;
use crate::texture::Texture;
use crate::error::Result;

mod gl3;
mod gl46;

pub trait OpenGlAbstraction {
    fn load_luts(textures: &[TextureConfig]) -> Result<FxHashMap<usize, Texture>>;
    fn clear_framebuffer<const REBIND: bool>(fbo: GLuint);
    fn create_framebuffer() -> GLuint;
    fn copy_framebuffer() -> GLuint;


}

pub use gl3::OpenGl3;
pub use gl46::OpenGl46;