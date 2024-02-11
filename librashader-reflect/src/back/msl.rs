use crate::back::targets::MSL;
use crate::back::{CompileShader, CompilerBackend, FromCompilation};
use crate::error::ShaderReflectError;
use crate::front::SpirvCompilation;
use crate::reflect::cross::msl::MslReflect;
use crate::reflect::cross::{CompiledProgram, SpirvCross};
use crate::reflect::naga::{Naga, NagaReflect};
use crate::reflect::ReflectShader;

/// Compiler options for MSL
#[derive(Debug, Default, Clone)]
pub struct MslNagaCompileOptions {
    // pub write_pcb_as_ubo: bool,
    pub sampler_bind_group: u32,
}

/// The context for a MSL compilation via spirv-cross.
pub struct CrossMslContext {
    /// The compiled HLSL program.
    pub artifact: CompiledProgram<spirv_cross::msl::Target>,
}

impl FromCompilation<SpirvCompilation, SpirvCross> for MSL {
    type Target = MSL;
    type Options = Option<spirv_cross::msl::Version>;
    type Context = CrossMslContext;
    type Output = impl CompileShader<Self::Target, Options = Self::Options, Context = Self::Context>
        + ReflectShader;

    fn from_compilation(
        compile: SpirvCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        Ok(CompilerBackend {
            backend: MslReflect::try_from(&compile)?,
        })
    }
}

impl FromCompilation<SpirvCompilation, Naga> for MSL {
    type Target = MSL;
    type Options = ();
    type Context = ();
    type Output = impl CompileShader<Self::Target, Options = Self::Options, Context = Self::Context>
        + ReflectShader;

    fn from_compilation(
        compile: SpirvCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        Ok(CompilerBackend {
            backend: NagaReflect::try_from(&compile)?,
        })
    }
}
