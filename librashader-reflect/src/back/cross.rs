use crate::back::targets::{GLSL, HLSL};
use crate::back::{CompileShader, CompilerBackend, FromCompilation};
use crate::error::ShaderReflectError;
use crate::front::SpirvCompilation;
use crate::reflect::cross::{CompiledProgram, GlslReflect, HlslReflect, SpirvCross};
use crate::reflect::{ReflectShader, ShaderOutputCompiler};

/// The GLSL version to target.
pub use spirv_cross::glsl::Version as GlslVersion;

/// The HLSL shader model version to target.
pub use spirv_cross::hlsl::ShaderModel as HlslShaderModel;

/// The context for a GLSL compilation via spirv-cross.
pub struct CrossGlslContext {
    /// A map of bindings of sampler names to binding locations.
    pub sampler_bindings: Vec<(String, u32)>,
    /// The compiled program artifact after compilation.
    pub artifact: CompiledProgram<spirv_cross::glsl::Target>,
}

impl FromCompilation<SpirvCompilation> for GLSL {
    type Target = GLSL;
    type Options = GlslVersion;
    type Context = CrossGlslContext;
    type Output = impl CompileShader<Self::Target, Options = GlslVersion, Context = Self::Context>
        + ReflectShader;

    fn from_compilation(
        compile: SpirvCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        let backend = SpirvCross::<GLSL>::create_reflection(compile)?;
        Ok(CompilerBackend {
            backend,
        })
    }
}

/// The context for a HLSL compilation via spirv-cross.
pub struct CrossHlslContext {
    /// The compiled HLSL program.
    pub artifact: CompiledProgram<spirv_cross::hlsl::Target>,
}

impl FromCompilation<SpirvCompilation> for HLSL {
    type Target = HLSL;
    type Options = Option<HlslShaderModel>;
    type Context = CrossHlslContext;
    type Output = impl CompileShader<Self::Target, Options = Self::Options, Context = Self::Context>
        + ReflectShader;

    fn from_compilation(
        compile: SpirvCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        Ok(CompilerBackend {
            backend: SpirvCross::<HLSL>::create_reflection(compile)?,
        })
    }
}
