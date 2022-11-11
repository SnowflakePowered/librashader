pub mod cross;
pub mod targets;

use std::fmt::Debug;
use rustc_hash::FxHashMap;
pub use targets::CompileShader;
use crate::reflect::semantics::{SemanticMap, TextureSemantics};
use crate::reflect::UniformSemantic;

#[derive(Debug)]
pub struct CompiledShader<Source, Context = ()> {
    pub vertex: Source,
    pub fragment: Source,
    pub context: Context,
}
