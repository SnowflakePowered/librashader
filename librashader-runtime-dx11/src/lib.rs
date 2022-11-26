#![feature(type_alias_impl_trait)]
#![feature(let_chains)]

mod filter_chain;

use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::back::{CompileShader, FromCompilation};
use librashader_reflect::front::shaderc::GlslangCompilation;
use rustc_hash::FxHashMap;
use std::error::Error;
use std::path::Path;

use librashader_reflect::reflect::semantics::{
    ReflectSemantics, SemanticMap, TextureSemantics, UniformSemantic, VariableSemantics,
};
use librashader_reflect::reflect::ReflectShader;

mod filter_pass;
#[cfg(test)]
mod hello_triangle;
mod texture;
mod util;

#[cfg(test)]
mod tests {
    use crate::hello_triangle::DXSample;
    use super::*;

    #[test]
    fn triangle_dx11() {
        let sample = hello_triangle::d3d11_hello_triangle::Sample::new().unwrap();
        let device = sample.device.clone();
        let chain = filter_chain::FilterChain::load_from_path(&device, "../test/slang-shaders/crt/crt-royale.slangp").unwrap();
        std::mem::forget(chain);
        hello_triangle::main(sample).unwrap();

    }
}
