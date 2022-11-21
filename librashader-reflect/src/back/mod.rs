pub mod cross;
pub mod targets;

use std::fmt::Debug;
use crate::back::targets::OutputTarget;
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::reflect::{ReflectShader, ShaderReflection};
use crate::reflect::semantics::ReflectSemantics;

#[derive(Debug)]
pub struct ShaderCompilerOutput<T, Context = ()> {
    pub vertex: T,
    pub fragment: T,
    pub context: Context,
}

pub trait CompileShader<T: OutputTarget> {
    type Options;
    type Context;

    fn compile(
        self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<T::Output, Self::Context>, ShaderCompileError>;
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

pub trait FromCompilation<T> {
    type Target: OutputTarget;
    type Options;
    type Context;

    fn from_compilation(
        compile: T,
    ) -> Result<
        CompilerBackend<impl CompileShader<Self::Target, Context = Self::Context> + ReflectShader>,
        ShaderReflectError,
    >;
}

pub struct CompilerBackend<T> {
    pub(crate) backend: T,
}

impl<T> ReflectShader for CompilerBackend<T>
where
    T: ReflectShader,
{
    fn reflect(
        &mut self,
        pass_number: usize,
        semantics: &ReflectSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError> {
        self.backend.reflect(pass_number, semantics)
    }
}
