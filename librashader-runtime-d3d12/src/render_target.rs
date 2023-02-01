use crate::texture::OutputTexture;

pub(crate) struct RenderTarget<'a> {
    pub x: f32,
    pub y: f32,
    pub mvp: &'a [f32; 16],
    pub output: OutputTexture,
}

