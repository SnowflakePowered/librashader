use crate::back::targets::WGSL;
use crate::back::wgsl::NagaWgslContext;
use crate::back::{CompileShader, ShaderCompilerOutput};
use crate::error::ShaderCompileError;
use crate::reflect::naga::{NagaLoweringOptions, NagaReflect};
use naga::back::wgsl::WriterFlags;
use naga::valid::{Capabilities, ModuleInfo, ValidationFlags, Validator};
use naga::Module;

impl CompileShader<WGSL> for NagaReflect {
    type Options = NagaLoweringOptions;
    type Context = NagaWgslContext;

    fn compile(
        mut self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, Self::Context>, ShaderCompileError> {
        let suppress = options.suppress_derivative_uniformity;
        fn write_wgsl(
            module: &Module,
            info: &ModuleInfo,
            suppress: bool,
        ) -> Result<String, ShaderCompileError> {
            let wgsl = naga::back::wgsl::write_string(&module, &info, WriterFlags::empty())?;
            // Naga doesn't re-emit diagnostics
            if suppress {
                Ok(format!(
                    "diagnostic(off, derivative_uniformity);\n{wgsl}"
                ))
            } else {
                Ok(wgsl)
            }
        }

        self.do_lowering(&options);

        // If we keep PCBs around we need the IMMEDIATES capability
        let capabilities = if options.write_pcb_as_ubo {
            Capabilities::empty()
        } else {
            Capabilities::IMMEDIATES
        };
        let mut valid = Validator::new(ValidationFlags::all(), capabilities);

        let vertex_info = valid.validate(&self.vertex)?;
        let fragment_info = valid.validate(&self.fragment)?;

        let fragment = write_wgsl(&self.fragment, &fragment_info, suppress)?;
        let vertex = write_wgsl(&self.vertex, &vertex_info, suppress)?;
        Ok(ShaderCompilerOutput {
            vertex,
            fragment,
            context: NagaWgslContext {
                fragment: self.fragment,
                vertex: self.vertex,
            },
        })
    }

    fn compile_boxed(
        self: Box<Self>,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, Self::Context>, ShaderCompileError> {
        <NagaReflect as CompileShader<WGSL>>::compile(*self, options)
    }
}
