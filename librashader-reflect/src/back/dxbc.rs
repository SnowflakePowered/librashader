use crate::back::spirv::WriteSpirV;
use crate::back::targets::{OutputTarget, DXBC};
use crate::back::{CompileShader, CompilerBackend, FromCompilation, ShaderCompilerOutput};
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::front::SpirvCompilation;
use crate::reflect::cross::glsl::GlslReflect;
use crate::reflect::cross::SpirvCross;
use crate::reflect::ReflectShader;
pub use spirv_to_dxil::ShaderModel;

use spirv_to_dxil::dxbc::{DxbcObject, RuntimeConfig};
use spirv_to_dxil::{BufferBinding, ShaderStage};
impl OutputTarget for DXBC {
    type Output = DxbcObject;
}

impl FromCompilation<SpirvCompilation, SpirvCross> for DXBC {
    type Target = DXBC;
    type Options = Option<ShaderModel>;
    type Context = ();
    type Output = impl CompileShader<Self::Target, Options = Self::Options, Context = Self::Context>
        + ReflectShader;

    fn from_compilation(
        compile: SpirvCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        let reflect = GlslReflect::try_from(&compile)?;
        Ok(CompilerBackend {
            // we can just reuse WriteSpirV as the backend.
            backend: WriteSpirV {
                reflect,
                vertex: compile.vertex,
                fragment: compile.fragment,
            },
        })
    }
}

impl CompileShader<DXBC> for WriteSpirV {
    type Options = Option<ShaderModel>;
    type Context = ();

    fn compile(
        self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<DxbcObject, Self::Context>, ShaderCompileError> {
        let sm = options.unwrap_or(ShaderModel::ShaderModel5_0);

        let config = RuntimeConfig {
            runtime_data_cbv: BufferBinding {
                register_space: 0,
                base_shader_register: u32::MAX,
            },
            push_constant_cbv: BufferBinding {
                register_space: 0,
                base_shader_register: 1,
            },
            shader_model_max: sm,
            ..RuntimeConfig::default()
        };

        // todo: do we want to allow other entry point names?
        let vertex = spirv_to_dxil::dxbc::spirv_to_dxbc(
            &self.vertex,
            None,
            "main",
            ShaderStage::Vertex,
            &config,
        )
        .map_err(ShaderCompileError::SpirvToDxilCompileError)?;

        let fragment = spirv_to_dxil::dxbc::spirv_to_dxbc(
            &self.fragment,
            None,
            "main",
            ShaderStage::Fragment,
            &config,
        )
        .map_err(ShaderCompileError::SpirvToDxilCompileError)?;

        Ok(ShaderCompilerOutput {
            vertex,
            fragment,
            context: (),
        })
    }
}
