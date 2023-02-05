pub use spirv_to_dxil::DxilObject;
pub use spirv_to_dxil::ShaderModel;
use spirv_to_dxil::{ShaderStage, ValidatorVersion};
use crate::back::{CompilerBackend, CompileShader, FromCompilation, ShaderCompilerOutput};
use crate::back::spirv::WriteSpirV;

use crate::back::targets::{DXIL, OutputTarget};
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::front::GlslangCompilation;
use crate::reflect::cross::GlslReflect;
use crate::reflect::ReflectShader;

impl OutputTarget for DXIL {
    type Output = DxilObject;
}

impl FromCompilation<GlslangCompilation> for DXIL {
    type Target = DXIL;
    type Options = Option<ShaderModel>;
    type Context = ();
    type Output = impl CompileShader<Self::Target, Options = Self::Options, Context = Self::Context>
    + ReflectShader;

    fn from_compilation(
        compile: GlslangCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        let vertex = compile.vertex.as_binary().to_vec();
        let fragment = compile.fragment.as_binary().to_vec();
        let reflect = GlslReflect::try_from(compile)?;
        Ok(CompilerBackend {
            // we can just reuse WriteSpirV as the backend.
            backend: WriteSpirV {
                reflect,
                vertex,
                fragment,
            },
        })
    }
}

impl CompileShader<DXIL> for WriteSpirV {
    type Options = Option<ShaderModel>;
    type Context = ();

    fn compile(
        self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<DxilObject, Self::Context>, ShaderCompileError> {
        let sm = options.unwrap_or(ShaderModel::ShaderModel6_0);

        let config = RuntimeConfig {
            runtime_data_cbv: ConstantBufferConfig {
                register_space: 0,
                base_shader_register: u32::MAX,
            },
            push_constant_cbv: ConstantBufferConfig {
                register_space: 0,
                base_shader_register: 1,
            },
            ..RuntimeConfig::default()
        };


        // todo: do we want to allow other entry point names?
        let vertex =
            spirv_to_dxil::spirv_to_dxil(&self.vertex,
                                         None, "main",
                                         ShaderStage::Vertex,
                                         sm,
                                         ValidatorVersion::None,
                                         Default::default())
                .map_err(|s| ShaderCompileError::SpirvToDxilCompileError(s))?;


        let fragment =
            spirv_to_dxil::spirv_to_dxil(&self.fragment,
                                         None, "main",
                                         ShaderStage::Fragment,
                                         ShaderModel::ShaderModel6_0,
                                         ValidatorVersion::None,
                                         Default::default())
                .map_err(|s| ShaderCompileError::SpirvToDxilCompileError(s))?;

        Ok(ShaderCompilerOutput {
            vertex,
            fragment,
            context: (),
        })
    }
}
