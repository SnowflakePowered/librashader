mod draw_quad;
mod framebuffer;
mod lut_load;
mod texture_bind;
mod ubo_ring;

#[cfg(test)]
pub mod hello_triangle;

use crate::gl::GLInterface;
use draw_quad::*;
use framebuffer::*;
use lut_load::*;
use texture_bind::*;
use ubo_ring::*;

pub struct DirectStateAccessGL;
impl GLInterface for DirectStateAccessGL {
    type Framebuffer = Gl46Framebuffer;
    type UboRing = Gl46UboRing<16>;
    type DrawQuad = Gl46DrawQuad;
    type LoadLut = Gl46LutLoad;
    type BindTexture = Gl46BindTexture;
}
