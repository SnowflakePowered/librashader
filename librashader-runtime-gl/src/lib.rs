#![feature(strict_provenance)]

mod hello_triangle;
mod filter_pass;
mod util;
mod framebuffer;
mod binding;
mod filter_chain;

use std::collections::HashMap;
use std::error::Error;
use std::iter::Filter;
use std::ops::Deref;
use std::path::Path;
use gl::types::{GLenum, GLint, GLsizei, GLsizeiptr, GLuint};
use glfw::Key::P;
use rustc_hash::FxHashMap;
use spirv_cross::spirv::Decoration;
use filter_pass::FilterPass;
use framebuffer::Framebuffer;

use librashader::{FilterMode, ShaderFormat, ShaderSource, WrapMode};
use librashader::image::Image;
use librashader_presets::{ShaderPassConfig, ShaderPreset};
use librashader_reflect::back::{CompileShader, ShaderCompilerOutput};
use librashader_reflect::back::cross::{GlslangGlslContext, GlVersion};
use librashader_reflect::back::targets::{FromCompilation, GLSL};
use librashader_reflect::front::shaderc::GlslangCompilation;
use librashader_reflect::reflect::cross::CrossReflect;
use librashader_reflect::reflect::{ReflectSemantics, ReflectShader, ShaderReflection, UniformSemantic};
use librashader_reflect::reflect::semantics::{MemberOffset, SemanticMap, TextureSemantics, UniformMeta, VariableMeta, VariableSemantics};
use librashader_reflect::reflect::{TextureSemanticMap, VariableSemanticMap};
use binding::{UniformLocation, VariableLocation};
use util::{GlImage, RingBuffer, Size, Texture, Viewport};
use crate::binding::UniformBinding;


#[cfg(test)]
mod tests {
    use crate::filter_chain::FilterChain;
    use super::*;

    #[test]
    fn triangle() {
        let (glfw, window, events, shader, vao) = hello_triangle::setup();
        let mut filter = FilterChain::load("../test/basic.slangp").unwrap();


        // FilterChain::load("../test/slang-shaders/crt/crt-royale.slangp").unwrap();

        hello_triangle::do_loop(glfw, window, events, shader, vao, &mut filter );
    }

    // #[test]
    // fn load_preset() {
    //
    //     load("../test/basic.slangp")
    //         .unwrap();
    // }
}
