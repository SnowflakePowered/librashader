use crate::back::targets::MSL;
use crate::back::{CompileShader, ShaderCompilerOutput};
use crate::error::ShaderCompileError;
use crate::reflect::naga::NagaReflect;

impl CompileShader<MSL> for NagaReflect {
    type Options = ();
    type Context = ();

    fn compile(
        self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, Self::Context>, ShaderCompileError> {
        todo!()
    }
}
