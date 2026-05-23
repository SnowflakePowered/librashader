use crate::back::targets::SPIRV;
use crate::back::{
    CompileReflectShader, CompileShader, CompilerBackend, FromCompilation, ShaderCompilerOutput,
};
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::front::SpirvCompilation;
#[cfg(feature = "cross")]
use crate::reflect::cross::glsl::GlslReflect;
#[cfg(feature = "cross")]
use crate::reflect::cross::SpirvCross;
use crate::reflect::naga::{Naga, NagaLoweringOptions, NagaReflect};
#[cfg(feature = "cross")]
use crate::reflect::semantics::ShaderSemantics;
#[cfg(feature = "cross")]
use crate::reflect::{ReflectShader, ShaderReflection};
use naga::Module;

#[cfg(feature = "cross")]
pub(crate) struct WriteSpirV {
    // rely on GLSL to provide out reflection but we don't actually need the AST.
    pub(crate) reflect: GlslReflect,
    pub(crate) vertex: Vec<u32>,
    pub(crate) fragment: Vec<u32>,
}

#[cfg(all(feature = "cross", feature = "nightly"))]
impl FromCompilation<SpirvCompilation, SpirvCross> for SPIRV {
    type Target = SPIRV;
    type Options = Option<()>;
    type Context = ();
    type Output = impl CompileReflectShader<Self::Target, SpirvCompilation, SpirvCross>;

    fn from_compilation(
        compile: SpirvCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        let reflect = GlslReflect::try_from(&compile)?;
        let vertex = compile.vertex;
        let fragment = compile.fragment;
        Ok(CompilerBackend {
            backend: WriteSpirV {
                reflect,
                vertex,
                fragment,
            },
        })
    }
}

#[cfg(all(feature = "cross", not(feature = "nightly")))]
impl FromCompilation<SpirvCompilation, SpirvCross> for SPIRV {
    type Target = SPIRV;
    type Options = Option<()>;
    type Context = ();
    type Output = Box<dyn CompileReflectShader<Self::Target, SpirvCompilation, SpirvCross> + Send>;

    fn from_compilation(
        compile: SpirvCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        let reflect = GlslReflect::try_from(&compile)?;
        let vertex = compile.vertex;
        let fragment = compile.fragment;
        Ok(CompilerBackend {
            backend: Box::new(WriteSpirV {
                reflect,
                vertex,
                fragment,
            }),
        })
    }
}

#[cfg(feature = "cross")]
impl ReflectShader for WriteSpirV {
    fn reflect(
        &mut self,
        pass_number: usize,
        semantics: &ShaderSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError> {
        self.reflect.reflect(pass_number, semantics)
    }

    fn validate(&mut self) -> Result<(), ShaderReflectError> {
        self.reflect.validate()
    }
}

#[cfg(feature = "cross")]
impl CompileShader<SPIRV> for WriteSpirV {
    type Options = Option<()>;
    type Context = ();

    fn compile(
        self,
        _options: Self::Options,
    ) -> Result<ShaderCompilerOutput<Vec<u32>, Self::Context>, ShaderCompileError> {
        Ok(ShaderCompilerOutput {
            vertex: self.vertex,
            fragment: self.fragment,
            context: (),
        })
    }

    fn compile_boxed(
        self: Box<Self>,
        _options: Self::Options,
    ) -> Result<ShaderCompilerOutput<Vec<u32>, Self::Context>, ShaderCompileError> {
        Ok(ShaderCompilerOutput {
            vertex: self.vertex,
            fragment: self.fragment,
            context: (),
        })
    }
}

/// The context for a SPIRV compilation via Naga
pub struct NagaSpirvContext {
    pub fragment: Module,
    pub vertex: Module,
}

#[cfg(feature = "nightly")]
impl FromCompilation<SpirvCompilation, Naga> for SPIRV {
    type Target = SPIRV;
    type Options = NagaSpirvOptions;
    type Context = NagaSpirvContext;
    type Output = impl CompileReflectShader<Self::Target, SpirvCompilation, Naga>;

    fn from_compilation(
        compile: SpirvCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        Ok(CompilerBackend {
            backend: NagaReflect::try_from(&compile)?,
        })
    }
}

#[cfg(not(feature = "nightly"))]
impl FromCompilation<SpirvCompilation, Naga> for SPIRV {
    type Target = SPIRV;
    type Options = NagaSpirvOptions;
    type Context = NagaSpirvContext;
    type Output = Box<dyn CompileReflectShader<Self::Target, SpirvCompilation, Naga> + Send>;

    fn from_compilation(
        compile: SpirvCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        Ok(CompilerBackend {
            backend: Box::new(NagaReflect::try_from(&compile)?),
        })
    }
}

pub struct NagaSpirvOptions {
    pub lowering: NagaLoweringOptions,
    pub version: (u8, u8),
}
