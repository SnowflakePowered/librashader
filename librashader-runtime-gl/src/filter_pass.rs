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

    fn build_vec4(location: Location<GLint>, buffer: &mut [u8], size: Size) {
        let vec4 = [size.width as f32, size.height as f32, 1.0 / size.width as f32, 1.0/ size.height as f32];
        if location.fragment >= 0 || location.vertex >= 0 {
            unsafe {
                if location.vertex >= 0 {
                    gl::Uniform4fv(location.vertex, 1, vec4.as_ptr());
                }
                if location.fragment >= 0 {
                    gl::Uniform4fv(location.fragment, 1, vec4.as_ptr());
                }
            }
        } else {
            let vec4 = bytemuck::cast_slice(&vec4);
            buffer.copy_from_slice(vec4);
        }
    }

    #[inline(always)]
    fn build_uniform<T>(location: Location<GLint>, buffer: &mut [u8], value: T, glfn: unsafe fn(GLint, T) -> ())
        where T: Copy, T: bytemuck::Pod
    {
        if location.fragment >= 0 || location.vertex >= 0 {
            unsafe {
                if location.vertex >= 0 {
                    glfn(location.vertex, value);
                }
                if location.fragment >= 0 {
                    glfn(location.fragment, value);
                }
            }
        } else {
            let mut buffer = bytemuck::cast_slice_mut(buffer);
            buffer[0] = value;
        }
    }

    fn build_uint(location: Location<GLint>, buffer: &mut [u8], value: u32) {
        Self::build_uniform(location, buffer, value, gl::Uniform1ui)
    }

    fn build_sint(location: Location<GLint>, buffer: &mut [u8], value: i32) {
        Self::build_uniform(location, buffer, value, gl::Uniform1i)
    }

    fn build_float(location: Location<GLint>, buffer: &mut [u8], value: f32) {
        Self::build_uniform(location, buffer, value, gl::Uniform1f)
    }

    // todo: build vec4 texture

    // framecount should be pre-modded
    fn build_semantics(&mut self, mvp: Option<&[f32]>, frame_count: u32, frame_direction: u32, fb_size: Size, vp_size: Size, original: &Texture, source: &Texture) {
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
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };
            FilterPass::build_vec4(location, &mut buffer[offset..][..4], fb_size)
            //
            // if location.fragment >= 0 || location.vertex >= 0 {
            //     FilterPass::build_vec4_uniform(location, fb_size);
            // } else {
            //     let (buffer, offset) = match variable.offset {
            //         MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
            //         MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            //     };
            //
            //     FilterPass::build_vec4(&mut buffer[offset..][..4], fb_size)
            // }
        }

        if let Some(variable) = self.reflection.meta.variable_meta.get(&VariableSemantics::FinalViewport) {
            // todo: do all variables have location..?
            let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };
            FilterPass::build_vec4(location, &mut buffer[offset..][..4], vp_size)
        }


        if let Some(variable) = self.reflection.meta.variable_meta.get(&VariableSemantics::FrameCount) {
            // todo: do all variables have location..?
            let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };
            FilterPass::build_uint(location, &mut buffer[offset..][..4], frame_count)
        }

        if let Some(variable) = self.reflection.meta.variable_meta.get(&VariableSemantics::FrameDirection) {
            // todo: do all variables have location..?
            let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };

            FilterPass::build_uint(location, &mut buffer[offset..][..4], frame_direction)
        }


    }
}