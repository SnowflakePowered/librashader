use librashader_common::Size;

/// A handle to an OpenGL texture with format and size information.
///
/// Generally for use as shader resource inputs.
#[derive(Default, Debug, Copy, Clone)]
pub struct GLImage {
    /// A GLuint to the texture.
    pub handle: Option<glow::Texture>,
    /// The format of the texture.
    pub format: u32,
    /// The size of the texture.
    pub size: Size<u32>,
}
