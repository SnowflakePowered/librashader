use crate::back::targets::{GLSL, HLSL};
use crate::back::{CompileShader, CompilerBackend, FromCompilation};
use crate::error::ShaderReflectError;
use crate::front::GlslangCompilation;
use crate::reflect::cross::{CompiledProgram, GlslReflect, HlslReflect};
use crate::reflect::ReflectShader;

/// The GLSL version to use.
pub type GlslVersion = spirv_cross::glsl::Version;

/// The HLSL shader model version to use
pub type HlslVersion = spirv_cross::hlsl::ShaderModel;

/// The context for a GLSL compilation via spirv-cross.
pub struct CrossGlslContext {
    /// A map of bindings of sampler names to binding locations.
    pub sampler_bindings: Vec<(String, u32)>,
    /// The compiled program artifact after compilation.
    pub artifact: CompiledProgram<spirv_cross::glsl::Target>,
}

impl FromCompilation<GlslangCompilation> for GLSL {
    type Target = GLSL;
    type Options = GlslVersion;
    type Context = CrossGlslContext;
    type Output = impl CompileShader<Self::Target, Options = GlslVersion, Context = Self::Context>
        + ReflectShader;

    fn from_compilation(
        compile: GlslangCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        Ok(CompilerBackend {
            backend: GlslReflect::try_from(compile)?,
        })
    }
}

/// The context for a HLSL compilation via spirv-cross.
pub struct CrossHlslContext {
    /// The compiled HLSL program.
    pub artifact: CompiledProgram<spirv_cross::hlsl::Target>,
}

impl FromCompilation<GlslangCompilation> for HLSL {
    type Target = HLSL;
    type Options = Option<HlslVersion>;
    type Context = CrossHlslContext;
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
