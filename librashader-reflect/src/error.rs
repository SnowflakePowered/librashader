use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShaderCompileError {
    #[error("shader")]
    NagaCompileError(Vec<naga::front::glsl::Error>),

    #[error("shaderc")]
    ShaderCCompileError(#[from] shaderc::Error),

    #[error("shaderc init")]
    ShaderCInitError,
}

#[derive(Error, Debug)]
pub enum ShaderReflectError {
    #[error("shader")]
    NagaCompileError(#[from] naga::front::spv::Error),
    #[error("spirv")]
    SpirvCrossError(#[from] spirv_cross::ErrorCode),
}

impl From<Vec<naga::front::glsl::Error>> for ShaderCompileError {
    fn from(err: Vec<naga::front::glsl::Error>) -> Self {
        ShaderCompileError::NagaCompileError(err)
    }
}
