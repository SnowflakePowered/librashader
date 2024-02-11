use naga::{Module, ResourceBinding};
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
        // https://github.com/libretro/RetroArch/blob/434e94c782af2e4d4277a24b7ed8e5fc54870088/gfx/drivers_shader/slang_process.cpp#L524
        todo!()
    }
}
