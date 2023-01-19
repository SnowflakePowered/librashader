use crate::error::ShaderCompileError;
use librashader_preprocess::ShaderSource;

#[cfg(feature = "unstable-naga")]
pub mod naga;

pub mod shaderc;

pub trait ShaderCompilation: Sized {
    /// Compile the input shader source file into a compilation unit.
    fn compile(source: &ShaderSource) -> Result<Self, ShaderCompileError>;
}
