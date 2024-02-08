use crate::error::ShaderCompileError;
use glslang::input::{CompilerOptions, ShaderInput};
use glslang::limits::ResourceLimits;
use librashader_preprocess::ShaderSource;

#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

/// A reflectable shader compilation via glslang.
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct GlslangCompilation {
    pub(crate) vertex: Vec<u32>,
    pub(crate) fragment: Vec<u32>,
}

impl GlslangCompilation {
    /// Tries to compile SPIR-V from the provided shader source.
    pub fn compile(source: &ShaderSource) -> Result<Self, ShaderCompileError> {
        compile_spirv(source)
    }
}

impl TryFrom<&ShaderSource> for GlslangCompilation {
    type Error = ShaderCompileError;

    /// Tries to compile SPIR-V from the provided shader source.
    fn try_from(source: &ShaderSource) -> Result<Self, Self::Error> {
        GlslangCompilation::compile(source)
    }
}

pub(crate) fn compile_spirv(
    source: &ShaderSource,
) -> Result<GlslangCompilation, ShaderCompileError> {
    let compiler = glslang::Compiler::acquire().ok_or(ShaderCompileError::CompilerInitError)?;

    let limits = ResourceLimits::default();
    let options = CompilerOptions::default();

    let vertex = glslang::input::ShaderSource::from(source.vertex.as_str());
    let vertex = ShaderInput::new(
        &vertex,
        &limits,
        glslang::ShaderStage::Vertex,
        &options,
        None,
    );
    let vertex = compiler.create_shader(vertex)?;

    let fragment = glslang::input::ShaderSource::from(source.fragment.as_str());
    let fragment = ShaderInput::new(
        &fragment,
        &limits,
        glslang::ShaderStage::Fragment,
        &options,
        None,
    );
    let fragment = compiler.create_shader(fragment)?;

    let vertex = Vec::from(vertex.compile()?);
    let fragment = Vec::from(fragment.compile()?);

    Ok(GlslangCompilation { vertex, fragment })
}

#[cfg(test)]
mod test {
    use crate::front::glslang::compile_spirv;
    use librashader_preprocess::ShaderSource;
    #[test]
    pub fn compile_shader() {
        let result = ShaderSource::load("../test/basic.slang").unwrap();
        let _spirv = compile_spirv(&result).unwrap();
    }
}
