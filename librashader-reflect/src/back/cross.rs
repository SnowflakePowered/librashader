use crate::back::targets::{CompilerBackend, FromCompilation, GLSL, HLSL};
use crate::back::CompileShader;
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::front::shaderc::GlslangCompilation;
use crate::reflect::cross::{GlslReflect, HlslReflect};
use crate::reflect::{ReflectShader, ShaderReflection};

pub type GlVersion = spirv_cross::glsl::Version;
impl FromCompilation<GlslangCompilation> for GLSL {
    type Target = GLSL;
    type Options = GlVersion;

    fn from_compilation(
        compile: GlslangCompilation,
    ) -> Result<
        CompilerBackend<impl CompileShader<Self::Target, Options = Self::Options> + ReflectShader>,
        ShaderReflectError,
    > {
        Ok(CompilerBackend {
            backend: GlslReflect::try_from(compile)?,
        })
    }
}

impl FromCompilation<GlslangCompilation> for HLSL {
    type Target = HLSL;
    type Options = Option<()>;

    fn from_compilation(
        compile: GlslangCompilation,
    ) -> Result<
        CompilerBackend<impl CompileShader<Self::Target, Options = Self::Options> + ReflectShader>,
        ShaderReflectError,
    > {
        Ok(CompilerBackend {
            backend: HlslReflect::try_from(compile)?,
        })
    }
}
