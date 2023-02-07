mod framebuffer;
pub(crate) mod gl3;
pub(crate) mod gl46;

use crate::binding::UniformLocation;
use crate::error::Result;
use crate::framebuffer::GLImage;
use crate::samplers::SamplerSet;
use crate::texture::InputTexture;
pub use framebuffer::Framebuffer;
use gl::types::{GLenum, GLuint};
use librashader_common::{ImageFormat, Size};
use librashader_presets::{Scale2D, TextureConfig};
use librashader_reflect::reflect::semantics::{TextureBinding, UboReflection};
use librashader_runtime::uniforms::UniformStorageAccess;
use rustc_hash::FxHashMap;

pub trait LoadLut {
    fn load_luts(textures: &[TextureConfig]) -> Result<FxHashMap<usize, InputTexture>>;
}

pub trait DrawQuad {
    fn new() -> Self;
    fn bind_vertices(&self);
    fn unbind_vertices(&self);
}

pub trait UboRing<const SIZE: usize> {
    fn new(buffer_size: u32) -> Self;
    fn bind_for_frame(
        &mut self,
        ubo: &UboReflection,
        ubo_location: &UniformLocation<GLuint>,
        storage: &impl UniformStorageAccess,
    );
}

pub trait FramebufferInterface {
    fn new(max_levels: u32) -> Framebuffer;
    fn scale(
        fb: &mut Framebuffer,
        scaling: Scale2D,
        format: ImageFormat,
        viewport_size: &Size<u32>,
        source_size: &Size<u32>,
        mipmap: bool,
    ) -> Result<Size<u32>>;
    fn clear<const REBIND: bool>(fb: &Framebuffer);
    fn copy_from(fb: &mut Framebuffer, image: &GLImage) -> Result<()>;
    fn init(fb: &mut Framebuffer, size: Size<u32>, format: impl Into<GLenum>) -> Result<()>;
}

pub trait BindTexture {
    fn bind_texture(samplers: &SamplerSet, binding: &TextureBinding, texture: &InputTexture);
    fn gen_mipmaps(texture: &InputTexture);
}

pub trait GLInterface {
    type FramebufferInterface: FramebufferInterface;
    type UboRing: UboRing<16>;
    type DrawQuad: DrawQuad;
    type LoadLut: LoadLut;
    type BindTexture: BindTexture;
}
