use crate::back::ShaderCompilerOutput;
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::reflect::{ReflectSemantics, ReflectShader, ShaderReflection};

pub trait OutputTarget {
    type Output;
}

pub struct GLSL;
pub struct HLSL;
pub struct SpirV;
pub struct MSL;

impl OutputTarget for GLSL {
    type Output = String;
}
impl OutputTarget for HLSL {
    type Output = String;
}
impl OutputTarget for SpirV {
    type Output = Vec<u32>;
}

pub struct CompilerBackend<T> {
    pub(crate) backend: T,
}

pub trait FromCompilation<T> {
    type Target: OutputTarget;
    type Options;
    type Context;

    fn from_compilation(
        compile: T,
    ) -> Result<CompilerBackend<impl CompileShader<Self::Target, Context=Self::Context> + ReflectShader>, ShaderReflectError>;
}

pub trait CompileShader<T: OutputTarget> {
    type Options;
    type Context;

    fn compile(
        self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<T::Output, Self::Context>, ShaderCompileError>;
}

impl<T> ReflectShader for CompilerBackend<T>
where
    T: ReflectShader,
{
    fn reflect(
        &mut self,
        pass_number: u32,
        semantics: &ReflectSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError> {
        self.backend.reflect(pass_number, semantics)
    }
}

impl<T, E> CompileShader<E> for CompilerBackend<T>
where
    T: CompileShader<E>,
    E: OutputTarget,
{
    type Options = T::Options;
    type Context = T::Context;

    fn compile(
        self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<E::Output, Self::Context>, ShaderCompileError> {
        self.backend.compile(options)
    }
}

mod test {
    use crate::back::targets::{FromCompilation, GLSL};
    use crate::front::shaderc::GlslangCompilation;
    pub fn huh(value: GlslangCompilation) {
        let _x = GLSL::from_compilation(value).unwrap();
    }
}
