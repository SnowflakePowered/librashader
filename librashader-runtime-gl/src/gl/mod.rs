pub(crate) mod gl3;
mod gl46;

use gl::types::{GLenum, GLint, GLsizei, GLuint};
use rustc_hash::FxHashMap;
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::{Scale2D, TextureConfig};
use librashader_reflect::reflect::semantics::{TextureBinding, UboReflection};
use librashader_runtime::uniforms::{UniformStorage, UniformStorageAccess};
use crate::binding::UniformLocation;
use crate::texture::Texture;
use crate::error::{FilterChainError, Result};
use crate::{GlImage, Viewport};
use crate::samplers::SamplerSet;

pub trait LoadLut {
    fn load_luts(textures: &[TextureConfig]) -> Result<FxHashMap<usize, Texture>>;
}

pub trait DrawQuad {
    fn new() -> Self;
    fn bind_vertices(&self);
    fn unbind_vertices(&self);
}

pub trait UboRing<const SIZE: usize> {
    fn new(buffer_size: u32) -> Self;
    fn bind_for_frame(&mut self, ubo: &UboReflection, ubo_location: &UniformLocation<GLuint>, storage: &impl UniformStorageAccess);
}

pub trait Framebuffer {
    fn new(max_levels: u32) -> Self;
    fn new_from_raw(
        texture: GLuint,
        handle: GLuint,
        format: GLenum,
        size: Size<u32>,
        miplevels: u32,
    ) -> Self;
    fn as_texture(&self, filter: FilterMode, wrap_mode: WrapMode) -> Texture;
    fn scale(
        &mut self,
        scaling: Scale2D,
        format: ImageFormat,
        viewport: &Viewport,
        _original: &Texture,
        source: &Texture,
    ) -> Result<Size<u32>>;
    fn clear<const REBIND: bool>(&self);
    fn copy_from(&mut self, image: &GlImage) -> Result<()>;
    fn init(&mut self, size: Size<u32>, format: impl Into<GLenum>) -> Result<()>;
}

pub trait BindTexture {
    fn bind_texture(samplers: &SamplerSet, binding: &TextureBinding, texture: &Texture);
}