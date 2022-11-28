use gl::types::GLint;
use librashader_reflect::reflect::semantics::{BindingStage, MemberOffset};

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
    // pub fn is_fragment_valid(&self) -> bool {
    //     self.fragment >= 0
    // }
    //
    // pub fn is_vertex_valid(&self) -> bool {
    //     self.vertex >= 0
    // }

    pub fn is_valid(&self, stage: BindingStage) -> bool {
        let mut validity = false;
        if stage.contains(BindingStage::FRAGMENT) {
            validity = validity || self.fragment >= 0;
        }
        if stage.contains(BindingStage::VERTEX) {
            validity = validity || self.vertex >= 0;
        }
        validity
    }
}
