use crate::error::ShaderCompileError;
use librashader_preprocess::ShaderSource;

use crate::front::{ShaderInputCompiler, ShaderReflectObject, WgslCompilation};

/// glslang compiler
pub struct NagaWgsl;

impl ShaderReflectObject for WgslCompilation {
    type Compiler = NagaWgsl;
}

impl TryFrom<&ShaderSource> for WgslCompilation {
    type Error = ShaderCompileError;

    /// Tries to compile  from the provided shader source.
    fn try_from(source: &ShaderSource) -> Result<Self, Self::Error> {
        NagaWgsl::compile(source)
    }
}

impl ShaderInputCompiler<WgslCompilation> for NagaWgsl {
    fn compile(source: &ShaderSource) -> Result<WgslCompilation, ShaderCompileError> {
        parse_wgsl(source)
    }
}

pub(crate) fn parse_wgsl(source: &ShaderSource) -> Result<WgslCompilation, ShaderCompileError> {
    let vertex: naga::Module = naga::front::wgsl::parse_str(&source.vertex)?;
    let fragment: naga::Module = naga::front::wgsl::parse_str(&source.fragment)?;

    Ok(WgslCompilation { vertex, fragment })
}
