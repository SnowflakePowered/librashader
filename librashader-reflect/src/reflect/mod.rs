use rustc_hash::FxHashMap;
use crate::error::ShaderReflectError;
use crate::reflect::semantics::{SemanticMap, ShaderReflection, VariableSemantics, TextureSemantics, VariableMeta, TextureMeta};

mod cross;
mod naga;
pub mod semantics;
mod rspirv;

pub trait ReflectShader {
    fn reflect(&self, options: &ReflectOptions) -> Result<ShaderReflection, ShaderReflectError>;
}

#[derive(Debug)]
pub enum UniformSemantic {
    Variable(SemanticMap<VariableSemantics>),
    Texture(SemanticMap<TextureSemantics>)
}

#[derive(Debug)]
pub struct ReflectOptions {
    pub pass_number: u32,
    pub uniform_semantics: FxHashMap<String, UniformSemantic>,
    pub non_uniform_semantics: FxHashMap<String, SemanticMap<TextureSemantics>>,
}

#[derive(Debug, Default)]
pub struct ReflectMeta {
    pub parameter_meta: FxHashMap<u32, VariableMeta>,
    pub variable_meta: FxHashMap<VariableSemantics, VariableMeta>,
    pub texture_meta: FxHashMap<SemanticMap<TextureSemantics>, TextureMeta>
}

pub fn builtin_uniform_semantics() -> FxHashMap<String, UniformSemantic> {
    let mut map = FxHashMap::default();

    map.insert("MVP".into(), UniformSemantic::Variable(SemanticMap {
        semantics: VariableSemantics::MVP,
        index: 0
    }));

    map.insert("OutputSize".into(), UniformSemantic::Variable(SemanticMap {
        semantics: VariableSemantics::Output,
        index: 0
    }));

    map.insert("FinalViewportSize".into(), UniformSemantic::Variable(SemanticMap {
        semantics: VariableSemantics::FinalViewport,
        index: 0
    }));

    map.insert("FrameCount".into(), UniformSemantic::Variable(SemanticMap {
        semantics: VariableSemantics::FrameCount,
        index: 0
    }));

    map.insert("FrameDirection".into(), UniformSemantic::Variable(SemanticMap {
        semantics: VariableSemantics::FrameDirection,
        index: 0
    }));
    map
}