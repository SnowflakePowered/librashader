use crate::error::{ShaderReflectError};
use crate::reflect::semantics::{
    SemanticMap, TextureImage, TextureSemantics, TextureSizeMeta, VariableMeta,
    VariableSemantics,
};
use rustc_hash::FxHashMap;

pub mod cross;
mod naga;
mod rspirv;
pub mod semantics;

pub trait ReflectShader {
    fn reflect(&mut self, pass_number: u32, semantics: &ReflectSemantics) -> Result<ShaderReflection, ShaderReflectError>;
}

#[derive(Debug)]
pub enum UniformSemantic {
    Variable(SemanticMap<VariableSemantics>),
    Texture(SemanticMap<TextureSemantics>),
}

#[derive(Debug)]
pub struct ReflectSemantics {
    pub uniform_semantics: FxHashMap<String, UniformSemantic>,
    pub non_uniform_semantics: FxHashMap<String, SemanticMap<TextureSemantics>>,
}

#[derive(Debug, Default)]
pub struct ReflectMeta {
    pub parameter_meta: FxHashMap<u32, VariableMeta>,
    pub variable_meta: FxHashMap<VariableSemantics, VariableMeta>,
    pub texture_meta: FxHashMap<SemanticMap<TextureSemantics>, TextureImage>,
    pub texture_size_meta: FxHashMap<SemanticMap<TextureSemantics>, TextureSizeMeta>,
}

pub use semantics::ShaderReflection;