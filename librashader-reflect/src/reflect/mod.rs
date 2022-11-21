use crate::error::ShaderReflectError;
use crate::reflect::semantics::{
    SemanticMap, TextureImage, TextureSemantics, TextureSizeMeta, VariableMeta, VariableSemantics,
};
use rustc_hash::FxHashMap;
use semantics::ReflectSemantics;

pub mod cross;

pub mod semantics;

#[cfg(feature = "unstable-rust-pipeline")]
mod naga;
#[cfg(feature = "unstable-rust-pipeline")]
mod rspirv;

pub trait ReflectShader {
    fn reflect(
        &mut self,
        pass_number: usize,
        semantics: &ReflectSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError>;
}

#[derive(Debug, Default)]
pub struct ReflectMeta {
    pub parameter_meta: FxHashMap<String, VariableMeta>,
    pub variable_meta: FxHashMap<VariableSemantics, VariableMeta>,
    pub texture_meta: FxHashMap<SemanticMap<TextureSemantics>, TextureImage>,
    pub texture_size_meta: FxHashMap<SemanticMap<TextureSemantics>, TextureSizeMeta>,
}

pub use semantics::ShaderReflection;
