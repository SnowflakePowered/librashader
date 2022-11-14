use std::iter::Filter;
use gl::types::{GLint, GLuint};
use librashader_reflect::back::cross::GlslangGlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::ShaderReflection;
use librashader_reflect::reflect::TextureSemanticMap;
use librashader_reflect::reflect::VariableSemanticMap;
use rustc_hash::FxHashMap;
use librashader_reflect::reflect::semantics::{MemberOffset, SemanticMap, TextureSemantics, VariableMeta, VariableSemantics};
use crate::framebuffer::Framebuffer;
use crate::util::{Location, VariableLocation, RingBuffer, Size, Texture};

pub struct FilterPass {
    pub reflection: ShaderReflection,
    pub compiled: ShaderCompilerOutput<String, GlslangGlslContext>,
    pub program: GLuint,
    pub ubo_location: Location<GLuint>,
    pub ubo_ring: Option<RingBuffer<GLuint, 16>>,
    pub uniform_buffer: Box<[u8]>,
    pub push_buffer: Box<[u8]>,
    pub locations: FxHashMap<String, VariableLocation>,
    pub framebuffer: Framebuffer,
    pub feedback_framebuffer: Framebuffer,
}

impl FilterPass {
    fn build_mvp(buffer: &mut [u8], mvp: &[f32]) {
        let mvp = bytemuck::cast_slice(mvp);
        buffer.copy_from_slice(mvp);
    }

    fn build_vec4(buffer: &mut [u8], width: u32, height: u32) {
        let vec4 = [width as f32, height as f32, 1.0 / width as f32, 1.0/ height as f32];
        let vec4 = bytemuck::cast_slice(&vec4);

        buffer.copy_from_slice(vec4);
    }

    fn build_vec4_uniform(location: Location<GLint>, width: u32, height: u32) {
        let vec4 = [width as f32, height as f32, 1.0 / width as f32, 1.0/ height as f32];
        unsafe {
            if location.vertex >= 0 {
                gl::Uniform4fv(location.vertex, 1, vec4.as_ptr());
            }
            if location.fragment >= 0 {
                gl::Uniform4fv(location.fragment, 1, vec4.as_ptr());
            }
        }
    }

    fn build_semantics(&mut self, mvp: Option<&[f32]>, fb_size: Size, vp_size: Size, original: &Texture, source: &Texture) {
        if let Some(variable) = self.reflection.meta.variable_meta.get(&VariableSemantics::MVP) {
            let mvp = mvp.unwrap_or(&[
                2f32, 0.0, 0.0, 0.0,
                0.0, 2.0, 0.0, 0.0,
                0.0, 0.0, 2.0, 0.0,
                -1.0, -1.0, 0.0, 1.0
            ]);

            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };
            FilterPass::build_mvp(&mut buffer[offset..][..mvp.len()], mvp)
        }

        if let Some(variable) = self.reflection.meta.variable_meta.get(&VariableSemantics::Output) {
            let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
            if location.fragment >= 0 || location.vertex >= 0 {
                FilterPass::build_vec4_uniform(location, fb_size.width, fb_size.height);
            } else {
                let (buffer, offset) = match variable.offset {
                    MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                    MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
                };

                FilterPass::build_vec4(&mut buffer[offset..][..4], fb_size.width, fb_size.height)
            }
        }
    }
}