use crate::error::ShaderReflectError;
use crate::reflect::semantics::ShaderReflection;

mod cross;
mod naga;
pub mod semantics;

pub trait ReflectShader {
    fn reflect(&self) -> Result<ShaderReflection, ShaderReflectError>;
}
