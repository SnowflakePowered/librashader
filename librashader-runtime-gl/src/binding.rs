use gl::types::GLint;
use librashader_reflect::reflect::semantics::{
    MemberOffset, SemanticMap, TextureSemantics, VariableSemantics,
};
use std::hash::Hash;

#[derive(Debug)]
pub enum VariableLocation {
    Ubo(UniformLocation<GLint>),
    Push(UniformLocation<GLint>),
}

impl VariableLocation {
    pub fn location(&self) -> UniformLocation<GLint> {
        match self {
            VariableLocation::Ubo(l) | VariableLocation::Push(l) => *l,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UniformLocation<T> {
    pub vertex: T,
    pub fragment: T,
}

impl UniformLocation<GLint> {
    pub fn is_fragment_valid(&self) -> bool {
        self.fragment >= 0
    }

    pub fn is_vertex_valid(&self) -> bool {
        self.vertex >= 0
    }

    pub fn is_valid(&self) -> bool {
        self.is_fragment_valid() || self.is_vertex_valid()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MemberLocation {
    Offset(MemberOffset),
    Uniform(UniformLocation<GLint>),
}

#[derive(Debug, Copy, Clone)]
pub struct TextureUnit<T>(T);

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum UniformBinding {
    Parameter(String),
    SemanticVariable(VariableSemantics),
    TextureSize(SemanticMap<TextureSemantics>),
}

impl From<VariableSemantics> for UniformBinding {
    fn from(value: VariableSemantics) -> Self {
        UniformBinding::SemanticVariable(value)
    }
}

impl From<SemanticMap<TextureSemantics>> for UniformBinding {
    fn from(value: SemanticMap<TextureSemantics>) -> Self {
        UniformBinding::TextureSize(value)
    }
}
