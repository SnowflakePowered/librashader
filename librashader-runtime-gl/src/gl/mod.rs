pub(crate) mod gl3;
pub(crate) mod gl46;

use crate::binding::UniformLocation;
use crate::error::Result;
use crate::framebuffer::{GLImage, Viewport};
use crate::samplers::SamplerSet;
use crate::texture::Texture;
use gl::types::{GLenum, GLuint};
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_presets::{Scale2D, TextureConfig};
use librashader_reflect::reflect::semantics::{TextureBinding, UboReflection};
use librashader_runtime::uniforms::UniformStorageAccess;
use rustc_hash::FxHashMap;

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
    fn bind_for_frame(
        &mut self,
        ubo: &UboReflection,
        ubo_location: &UniformLocation<GLuint>,
        storage: &impl UniformStorageAccess,
    );
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
        viewport: &Viewport<Self>,
        _original: &Texture,
        source: &Texture,
    ) -> Result<Size<u32>>;
    fn clear<const REBIND: bool>(&self);
    fn copy_from(&mut self, image: &GLImage) -> Result<()>;
    fn init(&mut self, size: Size<u32>, format: impl Into<GLenum>) -> Result<()>;
    fn handle(&self) -> GLuint;
    fn image(&self) -> GLuint;
    fn size(&self) -> Size<u32>;
    fn format(&self) -> GLenum;
}

pub trait BindTexture {
    fn bind_texture(samplers: &SamplerSet, binding: &TextureBinding, texture: &Texture);
}

pub trait GLInterface {
    type Framebuffer: Framebuffer;
    type UboRing: UboRing<16>;
    type DrawQuad: DrawQuad;
    type LoadLut: LoadLut;
    type BindTexture: BindTexture;
}
