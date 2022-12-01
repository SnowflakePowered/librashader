use gl::types::{GLenum, GLuint};
use librashader_common::Size;

#[derive(Default, Debug, Copy, Clone)]
pub struct GLImage {
    pub handle: GLuint,
    pub format: GLenum,
    pub size: Size<u32>,
    pub padded_size: Size<u32>,
}
