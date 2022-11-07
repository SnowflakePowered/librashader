pub mod targets;
mod cross;

use std::fmt::Debug;
pub use targets::ShaderCompiler;

#[derive(Debug)]
pub struct CompiledShader<T> {
    pub vertex: T,
    pub fragment: T,
}
