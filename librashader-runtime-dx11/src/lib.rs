#![feature(type_alias_impl_trait)]

mod filter_chain;


use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::back::{CompileShader, FromCompilation};
use rustc_hash::FxHashMap;
use std::error::Error;
use std::path::Path;
use librashader_reflect::front::shaderc::GlslangCompilation;

use librashader_reflect::reflect::semantics::{ReflectSemantics, SemanticMap, TextureSemantics, UniformSemantic, VariableSemantics};
use librashader_reflect::reflect::ReflectShader;

#[cfg(test)]
mod hello_triangle;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_dx11() {
        hello_triangle::main().unwrap();
    }
}
