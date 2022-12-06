pub mod cross;
pub mod targets;
mod spirv;

use crate::back::targets::OutputTarget;
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::reflect::semantics::ShaderSemantics;
use crate::reflect::{ReflectShader, ShaderReflection};
use std::fmt::Debug;

/// The output of the shader compiler.
#[derive(Debug)]
pub struct ShaderCompilerOutput<T, Context = ()> {
    /// The output for the vertex shader.
    pub vertex: T,
    /// The output for the fragment shader.
    pub fragment: T,
    /// Additional context provided by the shader compiler.
    pub context: Context,
}

/// A trait for objects that can be compiled into a shader.
pub trait CompileShader<T: OutputTarget> {
    /// Options provided to the compiler.
    type Options;
    /// Additional context returned by the compiler after compilation.
    type Context;

    /// Consume the object and return the compiled output of the shader.
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

/// A trait for reflectable compilations that can be transformed into an object ready for reflection or compilation.
pub trait FromCompilation<T> {
    /// The target that the transformed object is expected to compile for.
    type Target: OutputTarget;
    /// Options provided to the compiler.
    type Options;
    /// Additional context returned by the compiler after compilation.
    type Context;

    /// The output type after conversion.
    type Output: CompileShader<Self::Target, Context = Self::Context, Options = Self::Options>
        + ReflectShader;

    /// Tries to convert the input object into an object ready for compilation.
    fn from_compilation(compile: T) -> Result<CompilerBackend<Self::Output>, ShaderReflectError>;
}

/// A wrapper for a compiler backend.
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
        semantics: &ShaderSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError> {
        self.backend.reflect(pass_number, semantics)
    }
}
