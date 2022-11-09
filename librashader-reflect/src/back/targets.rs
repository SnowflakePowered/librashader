use crate::back::CompiledShader;
use crate::error::ShaderCompileError;
use crate::reflect::ShaderReflection;

pub trait OutputTarget { }

pub struct GLSL;
pub struct HLSL;
pub struct SpirV;
pub struct MSL;

impl OutputTarget for GLSL {}
impl OutputTarget for HLSL {}
impl OutputTarget for SpirV {}
impl OutputTarget for MSL {}

pub trait ShaderCompiler<T: OutputTarget> {
    type Output;
    type Options;
    type Context = ();

    fn compile(&mut self, options: &Self::Options, reflection: &ShaderReflection) -> Result<CompiledShader<Self::Output, Self::Context>, ShaderCompileError>;
}
