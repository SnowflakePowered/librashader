use crate::reflect::semantics::MemberOffset;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShaderCompileError {
    #[error("shader")]
    NagaCompileError(Vec<naga::front::glsl::Error>),

    #[error("shaderc")]
    ShaderCCompileError(#[from] shaderc::Error),

    #[error("shaderc init")]
    ShaderCInitError,

    #[error("cross")]
    SpirvCrossCompileError(#[from] spirv_cross::ErrorCode),
}

#[derive(Debug)]
pub enum SemanticsErrorKind {
    InvalidUniformBufferCount(usize),
    InvalidPushBufferSize(u32),
    InvalidLocation(u32),
    InvalidDescriptorSet(u32),
    InvalidInputCount(usize),
    InvalidOutputCount(usize),
    InvalidBinding(u32),
    InvalidResourceType,
    InvalidRange(u32),
    UnknownSemantics(String),
    InvalidTypeForSemantic(String),
}

#[derive(Error, Debug)]
pub enum ShaderReflectError {
    #[error("shader")]
    NagaCompileError(#[from] naga::front::spv::Error),
    #[error("spirv")]
    SpirvCrossError(#[from] spirv_cross::ErrorCode),
    #[error("rspirv")]
    RspirvParseError(#[from] rspirv::binary::ParseState),
    #[error("error when verifying vertex semantics")]
    VertexSemanticError(SemanticsErrorKind),
    #[error("error when verifying texture semantics")]
    FragmentSemanticError(SemanticsErrorKind),
    #[error("vertx and fragment shader must have same binding")]
    MismatchedUniformBuffer { vertex: u32, fragment: u32 },
    #[error("filter chain is non causal")]
    NonCausalFilterChain { pass: usize, target: usize },
    #[error("mismatched offset")]
    MismatchedOffset {
        semantic: String,
        vertex: MemberOffset,
        fragment: MemberOffset,
    },
    #[error("mismatched component")]
    MismatchedComponent {
        semantic: String,
        vertex: u32,
        fragment: u32,
    },
    #[error("the binding is already in use")]
    BindingInUse(u32),
}

impl From<Vec<naga::front::glsl::Error>> for ShaderCompileError {
    fn from(err: Vec<naga::front::glsl::Error>) -> Self {
        ShaderCompileError::NagaCompileError(err)
    }
}
