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

pub struct DirectStateAccessGL;
impl GLInterface for DirectStateAccessGL {
    type Framebuffer = Gl46Framebuffer;
    type UboRing = Gl46UboRing<16>;
    type DrawQuad = Gl46DrawQuad;
    type LoadLut = Gl46LutLoad;
    type BindTexture = Gl46BindTexture;
}