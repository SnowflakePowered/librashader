pub mod targets;
mod cross;

use std::fmt::Debug;
pub use targets::ShaderCompiler;

#[derive(Debug)]
pub struct CompiledShader<Source, Context = ()> {
    pub vertex: Source,
    pub fragment: Source,
    pub context: Context,
}

