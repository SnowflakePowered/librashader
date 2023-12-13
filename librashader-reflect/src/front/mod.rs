use crate::error::ShaderCompileError;
use librashader_preprocess::ShaderSource;

mod shaderc;

pub use crate::front::shaderc::GlslangCompilation;

/// Trait for types that can compile shader sources into a compilation unit.
pub trait ShaderCompilation: Sized {
    /// Compile the input shader source file into a compilation unit.
    fn compile(source: &ShaderSource) -> Result<Self, ShaderCompileError>;
}

impl<T: for<'a> TryFrom<&'a ShaderSource, Error = ShaderCompileError>> ShaderCompilation for T {
    fn compile(source: &ShaderSource) -> Result<Self, ShaderCompileError> {
        source.try_into()
    }
}
