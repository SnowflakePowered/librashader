mod lut_load;
mod draw_quad;
mod ubo_ring;
mod framebuffer;
mod texture_bind;
#[cfg(test)]
pub mod hello_triangle;

use lut_load::*;
use draw_quad::*;
use ubo_ring::*;
use framebuffer::*;
use texture_bind::*;
use crate::gl::GLInterface;

pub struct CompatibilityGL;
impl GLInterface for CompatibilityGL {
    type Framebuffer = Gl3Framebuffer;
    type UboRing = Gl3UboRing<16>;
    type DrawQuad = Gl3DrawQuad;
    type LoadLut = Gl3LutLoad;
    type BindTexture = Gl3BindTexture;
}
