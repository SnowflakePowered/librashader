use crate::error::ShaderCompileError;
use librashader_preprocess::ShaderSource;
pub(crate) mod spirv_passes;

#[cfg(feature = "glslang-in")]
mod glslang;

#[cfg(feature = "glslang-in")]
pub use crate::front::glslang::Glslang;

use crate::reflect::semantics::ShaderSemantics;

#[cfg(feature = "naga-in")]
mod naga;

#[cfg(feature = "naga-in")]
pub use crate::front::naga::NagaWgsl;

/// The output of a shader compiler that is reflectable.
pub trait ShaderReflectObject: Sized {
    /// The compiler that produces this reflect object.
    type Compiler;
}

/// Trait for types that can compile shader sources into a compilation unit.
pub trait ShaderInputCompiler<O: ShaderReflectObject>: Sized {
    /// Compile the input shader source file into a compilation unit.
    fn compile(source: &ShaderSource) -> Result<O, ShaderCompileError>;

    /// Apply the mangled semantics if needed for the outputs
    fn apply_mangled_semantics(_semantics: &mut ShaderSemantics) {}
}

/// A reflectable shader compilation via glslang.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpirvCompilation {
    pub(crate) vertex: Vec<u32>,
    pub(crate) fragment: Vec<u32>,
}

/// A reflectable shader compilation via naga, where the input is WGSL, and not GLSL.
///
/// This is only used for .wgsl.slangpacks when running in `wasm32-unknown-unknown`.
#[derive(Debug, Clone)]
#[cfg(feature = "naga-in")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WgslCompilation {
    pub(crate) vertex: ::naga::Module,
    pub(crate) fragment: ::naga::Module,
}
