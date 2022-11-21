use crate::back::targets::{CompilerBackend, FromCompilation, GLSL, HLSL};
use crate::back::CompileShader;
use crate::error::ShaderReflectError;
use crate::front::shaderc::GlslangCompilation;
use crate::reflect::cross::{CompiledAst, GlslReflect, HlslReflect};
use crate::reflect::ReflectShader;

pub type GlVersion = spirv_cross::glsl::Version;
pub struct GlslangGlslContext {
    pub sampler_bindings: Vec<u32>,
    pub compiler: CompiledAst<spirv_cross::glsl::Target>,
}
impl FromCompilation<GlslangCompilation> for GLSL {
    type Target = GLSL;
    type Options = GlVersion;
    type Context = GlslangGlslContext;

    fn from_compilation(
        compile: GlslangCompilation,
    ) -> Result<
        CompilerBackend<
            impl CompileShader<Self::Target, Options = Self::Options, Context = Self::Context>
                + ReflectShader,
        >,
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
    type Context = ();

    fn from_compilation(
        compile: GlslangCompilation,
    ) -> Result<
        CompilerBackend<
            impl CompileShader<Self::Target, Options = Self::Options, Context = Self::Context>
                + ReflectShader,
        >,
        ShaderReflectError,
    > {
        Ok(CompilerBackend {
            backend: HlslReflect::try_from(compile)?,
        })
    }
}
