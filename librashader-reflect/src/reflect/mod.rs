use crate::error::ShaderReflectError;
use crate::reflect::semantics::{
    SemanticMap, TextureImage, TextureSemantics, TextureSizeMeta, VariableMeta, VariableSemantics,
};
use rustc_hash::FxHashMap;
use std::str::FromStr;

pub mod cross;
mod naga;
mod rspirv;
pub mod semantics;

pub trait ReflectShader {
    fn reflect(
        &mut self,
        pass_number: u32,
        semantics: &ReflectSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError>;
}

pub trait TextureSemanticMap<T> {
    fn get_texture_semantic(&self, name: &str) -> Option<SemanticMap<TextureSemantics>>;
}

pub trait VariableSemanticMap<T> {
    fn get_variable_semantic(&self, name: &str) -> Option<SemanticMap<VariableSemantics>>;
}

impl VariableSemanticMap<UniformSemantic> for FxHashMap<String, UniformSemantic> {
    fn get_variable_semantic(&self, name: &str) -> Option<SemanticMap<VariableSemantics>> {
        match self.get(name) {
            // existing uniforms in the semantic map have priority
            None => match name {
                "MVP" => Some(SemanticMap {
                    semantics: VariableSemantics::MVP,
                    index: 0,
                }),
                "OutputSize" => Some(SemanticMap {
                    semantics: VariableSemantics::Output,
                    index: 0,
                }),
                "FinalViewportSize" => Some(SemanticMap {
                    semantics: VariableSemantics::FinalViewport,
                    index: 0,
                }),
                "FrameCount" => Some(SemanticMap {
                    semantics: VariableSemantics::FrameCount,
                    index: 0,
                }),
                "FrameDirection" => Some(SemanticMap {
                    semantics: VariableSemantics::FrameDirection,
                    index: 0,
                }),
                _ => None,
            },
            Some(UniformSemantic::Variable(variable)) => Some(*variable),
            Some(UniformSemantic::Texture(_)) => None,
        }
    }
}

impl TextureSemanticMap<UniformSemantic> for FxHashMap<String, UniformSemantic> {
    fn get_texture_semantic(&self, name: &str) -> Option<SemanticMap<TextureSemantics>> {
        match self.get(name) {
            None => {
                if let Some(semantics) = TextureSemantics::TEXTURE_SEMANTICS
                    .iter()
                    .find(|f| name.starts_with(f.size_uniform_name()))
                {
                    if semantics.is_array() {
                        let index = &name[semantics.size_uniform_name().len()..];
                        let Ok(index) = u32::from_str(index) else {
                            return None;
                        };
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index,
                        });
                    } else if name == semantics.size_uniform_name() {
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index: 0,
                        });
                    }
                }
                None
            }
            Some(UniformSemantic::Variable(_)) => None,
            Some(UniformSemantic::Texture(texture)) => Some(*texture),
        }
    }
}

impl TextureSemanticMap<UniformSemantic> for FxHashMap<String, SemanticMap<TextureSemantics>> {
    fn get_texture_semantic(&self, name: &str) -> Option<SemanticMap<TextureSemantics>> {
        match self.get(name) {
            None => {
                if let Some(semantics) = TextureSemantics::TEXTURE_SEMANTICS
                    .iter()
                    .find(|f| name.starts_with(f.texture_name()))
                {
                    if semantics.is_array() {
                        let index = &name[semantics.texture_name().len()..];
                        let Ok(index) = u32::from_str(index) else {return None};
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index,
                        });
                    } else if name == semantics.texture_name() {
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index: 0,
                        });
                    }
                }
                None
            }
            Some(texture) => Some(*texture),
        }
    }
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
