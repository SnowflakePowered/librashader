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

#[derive(Debug)]
pub enum SemanticsErrorKind {
    InvalidUniformBufferSize(usize),
    InvalidPushBufferSize(usize),
    InvalidLocation(u32),
    InvalidDescriptorSet(u32),
    InvalidInputCount(usize),
    InvalidOutputCount(usize),
    InvalidBinding(u32),
    InvalidResourceType,
}

#[derive(Error, Debug)]
pub enum ShaderReflectError {
    #[error("shader")]
    NagaCompileError(#[from] naga::front::spv::Error),
    #[error("spirv")]
    SpirvCrossError(#[from] spirv_cross::ErrorCode),
    #[error("error when verifying vertex semantics")]
    VertexSemanticError(SemanticsErrorKind),
    #[error("error when verifying texture semantics")]
    FragmentSemanticError(SemanticsErrorKind),
    #[error("vertx and fragment shader must have same binding")]
    MismatchedUniformBuffer { vertex: Option<u32>, fragment: Option<u32> },
    #[error("binding exceeded max")]
    InvalidBinding(u32)
}

impl From<Vec<naga::front::glsl::Error>> for ShaderCompileError {
    fn from(err: Vec<naga::front::glsl::Error>) -> Self {
        ShaderCompileError::NagaCompileError(err)
    }
}
