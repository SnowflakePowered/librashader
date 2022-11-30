use crate::back::targets::{GLSL, HLSL};
use crate::back::{CompileShader, CompilerBackend, FromCompilation};
use crate::error::ShaderReflectError;
use crate::front::shaderc::GlslangCompilation;
use crate::reflect::cross::{CompiledProgram, GlslReflect, HlslReflect};
use crate::reflect::ReflectShader;

pub type GlVersion = spirv_cross::glsl::Version;
pub struct GlslangGlslContext {
    pub sampler_bindings: Vec<(String, u32)>,
    pub compiler: CompiledProgram<spirv_cross::glsl::Target>,
}

impl FromCompilation<GlslangCompilation> for GLSL {
    type Target = GLSL;
    type Options = GlVersion;
    type Context = GlslangGlslContext;
    type Output = impl CompileShader<Self::Target, Options = GlVersion, Context = GlslangGlslContext>
        + ReflectShader;

    fn from_compilation(
        compile: GlslangCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        Ok(CompilerBackend {
            backend: GlslReflect::try_from(compile)?,
        })
    }
}

pub struct GlslangHlslContext {
    pub compiler: CompiledProgram<spirv_cross::hlsl::Target>,
}

impl FromCompilation<GlslangCompilation> for HLSL {
    type Target = HLSL;
    type Options = Option<()>;
    type Context = GlslangHlslContext;
    type Output = impl CompileShader<Self::Target, Options = Self::Options, Context = Self::Context>
        + ReflectShader;

    fn from_compilation(
        compile: GlslangCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        Ok(CompilerBackend {
            backend: HlslReflect::try_from(compile)?,
        })
    }
}
