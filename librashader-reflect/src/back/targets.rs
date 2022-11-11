use crate::back::CompiledShader;
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::reflect::{ReflectSemantics, ReflectShader, ShaderReflection};
use std::marker::PhantomData;

pub trait OutputTarget {
    type Output;
    type AdditionalContext;
}

pub struct GLSL;
pub struct HLSL;
pub struct SpirV;
pub struct MSL;

impl OutputTarget for GLSL {
    type Output = String;
    type AdditionalContext = Vec<u32>;
}
impl OutputTarget for HLSL {
    type Output = String;
    type AdditionalContext = ();
}
impl OutputTarget for SpirV {
    type Output = Vec<u32>;
    type AdditionalContext = ();
}

pub struct CompilerBackend<T> {
    pub(crate) backend: T,
}

pub trait FromCompilation<T> {
    type Target: OutputTarget;
    type Options;
    fn from_compilation(
        compile: T,
    ) -> Result<CompilerBackend<impl CompileShader<Self::Target> + ReflectShader>, ShaderReflectError>;
}

pub trait CompileShader<T: OutputTarget> {
    type Options;
    fn compile(
        &mut self,
        options: Self::Options,
    ) -> Result<CompiledShader<T::Output, T::AdditionalContext>, ShaderCompileError>;
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

    fn compile(
        &mut self,
        options: Self::Options,
    ) -> Result<CompiledShader<E::Output, E::AdditionalContext>, ShaderCompileError> {
        self.backend.compile(options)
    }
}

mod test {
    use crate::back::targets::{CompilerBackend, FromCompilation, GLSL};
    use crate::front::shaderc::GlslangCompilation;
    pub fn huh(value: GlslangCompilation) {
        let x = GLSL::from_compilation(value).unwrap();
    }
}

//
// impl ReflectShader for GLSL {
//     fn reflect(&mut self, pass_number: u32, semantics: &ReflectSemantics) -> Result<ShaderReflection, ShaderReflectError> {
//         self.0.reflect(pass_number, semantics)
//     }
// }
//
// impl ShaderCompiler<GLSL> for GLSL {
//     type Output = String;
//     type Context = Vec<u32>;
//     fn compile(&mut self, options: Self::Options) -> Result<CompiledShader<Self::Output, Self::Context>, ShaderCompileError> {
//         self.0.compile(options)
//     }
// }
//
// impl TryFrom<GlslangCompilation> for GLSL {
//     type Error = ShaderReflectError;
//
//     fn try_from(value: GlslangCompilation) -> Result<Self, Self::Error> {
//         let value = GlslReflect::try_from(value)?;
//         Ok(Self(value))
//     }
// }
//
// impl ReflectShader for HLSL {
//     fn reflect(&mut self, pass_number: u32, semantics: &ReflectSemantics) -> Result<ShaderReflection, ShaderReflectError> {
//         self.0.reflect(pass_number, semantics)
//     }
// }
//
// impl ShaderCompiler<HLSL> for HLSL {
//     type Output = String;
//     fn compile(&mut self, options: Self::Options) -> Result<CompiledShader<Self::Output, Self::Context>, ShaderCompileError> {
//         self.0.compile(options)
//     }
// }
//
// impl TryFrom<GlslangCompilation> for HLSL {
//     type Error = ShaderReflectError;
//
//     fn try_from(value: GlslangCompilation) -> Result<Self, Self::Error> {
//         let value = HlslReflect::try_from(value)?;
//         Ok(Self(value))
//     }
// }
