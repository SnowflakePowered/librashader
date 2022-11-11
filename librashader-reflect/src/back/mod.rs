pub mod cross;
pub mod targets;

use std::fmt::Debug;

pub use targets::CompileShader;

#[derive(Debug)]
pub struct ShaderCompilerOutput<T, Context = ()> {
    pub vertex: T,
    pub fragment: T,
    pub context: Context,
}
