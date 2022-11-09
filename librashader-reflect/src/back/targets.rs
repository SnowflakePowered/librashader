use crate::back::CompiledShader;
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::front::shaderc::GlslangCompilation;
use crate::reflect::{ReflectSemantics, ReflectShader, ShaderReflection};
use crate::reflect::cross::{GlslReflect, HlslReflect};

pub trait OutputTarget { }

pub struct GLSL(GlslReflect);
pub struct HLSL(HlslReflect);
pub struct SpirV;
pub struct MSL;

impl OutputTarget for GLSL {}
impl OutputTarget for HLSL {}
impl OutputTarget for SpirV {}
impl OutputTarget for MSL {}

pub trait ShaderCompiler<T: OutputTarget>: ReflectShader {
    type Output;
    type Options = Option<()>;
    type Context = ();

    fn compile(&mut self, options: Self::Options) -> Result<CompiledShader<Self::Output, Self::Context>, ShaderCompileError>;
}

impl ReflectShader for GLSL {
    fn reflect(&mut self, pass_number: u32, semantics: &ReflectSemantics) -> Result<ShaderReflection, ShaderReflectError> {
        self.0.reflect(pass_number, semantics)
    }
}

impl ShaderCompiler<GLSL> for GLSL {
    type Output = String;
    type Context = Vec<u32>;
    fn compile(&mut self, options: Self::Options) -> Result<CompiledShader<Self::Output, Self::Context>, ShaderCompileError> {
        self.0.compile(options)
    }
}

impl TryFrom<GlslangCompilation> for GLSL {
    type Error = ShaderReflectError;

    fn try_from(value: GlslangCompilation) -> Result<Self, Self::Error> {
        let value = GlslReflect::try_from(value)?;
        Ok(Self(value))
    }
}

impl ReflectShader for HLSL {
    fn reflect(&mut self, pass_number: u32, semantics: &ReflectSemantics) -> Result<ShaderReflection, ShaderReflectError> {
        self.0.reflect(pass_number, semantics)
    }
}

impl ShaderCompiler<HLSL> for HLSL {
    type Output = String;
    fn compile(&mut self, options: Self::Options) -> Result<CompiledShader<Self::Output, Self::Context>, ShaderCompileError> {
        self.0.compile(options)
    }
}

impl TryFrom<GlslangCompilation> for HLSL {
    type Error = ShaderReflectError;

    fn try_from(value: GlslangCompilation) -> Result<Self, Self::Error> {
        let value = HlslReflect::try_from(value)?;
        Ok(Self(value))
    }
}
