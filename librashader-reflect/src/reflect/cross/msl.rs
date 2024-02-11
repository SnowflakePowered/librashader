use crate::back::msl::CrossMslContext;
use crate::back::targets::MSL;
use crate::back::{CompileShader, ShaderCompilerOutput};
use crate::error::ShaderCompileError;
use crate::reflect::cross::{CompiledAst, CompiledProgram, CrossReflect};
use spirv_cross::msl;

pub(crate) type MslReflect = CrossReflect<spirv_cross::msl::Target>;

impl CompileShader<MSL> for CrossReflect<spirv_cross::msl::Target> {
    type Options = Option<spirv_cross::msl::Version>;
    type Context = CrossMslContext;

    fn compile(
        mut self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, CrossMslContext>, ShaderCompileError> {
        let version = options.unwrap_or(msl::Version::V2_0);
        let mut options = spirv_cross::msl::CompilerOptions::default();
        options.version = version;

        // This is actually all sorts of broken because there's no way to change bindings
        // with the current version of spirv_cross.

        self.vertex.set_compiler_options(&options)?;
        self.fragment.set_compiler_options(&options)?;

        Ok(ShaderCompilerOutput {
            vertex: self.vertex.compile()?,
            fragment: self.fragment.compile()?,
            context: CrossMslContext {
                artifact: CompiledProgram {
                    vertex: CompiledAst(self.vertex),
                    fragment: CompiledAst(self.fragment),
                },
            },
        })
    }
}
