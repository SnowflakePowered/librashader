use std::iter::Filter;
use gl::types::{GLenum, GLint, GLsizei, GLsizeiptr, GLuint};
use librashader_reflect::back::cross::GlslangGlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::ShaderReflection;
use librashader_reflect::reflect::TextureSemanticMap;
use librashader_reflect::reflect::VariableSemanticMap;
use rustc_hash::FxHashMap;
use librashader::{ShaderFormat, ShaderSource};
use librashader_presets::{Scale2D, ScaleType, Scaling, ShaderPassConfig, ShaderPreset};
use librashader_reflect::reflect::semantics::{MemberOffset, SemanticMap, TextureImage, TextureSemantics, VariableMeta, VariableSemantics};
use crate::{FilterChain, FilterCommon};
use crate::framebuffer::Framebuffer;
use crate::util::{Location, VariableLocation, RingBuffer, Size, GlImage, Texture, Viewport};

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
    pub source: ShaderSource,
    pub config: ShaderPassConfig
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

    fn set_texture(binding: &TextureImage, texture: &Texture) {
        unsafe {
            // eprintln!("binding {} = texture {}", binding.binding, texture.image.handle);
            gl::ActiveTexture((gl::TEXTURE0 + binding.binding) as GLenum);
            gl::BindTexture(gl::TEXTURE_2D, texture.image.handle);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, GLenum::from(texture.filter) as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, texture.filter.gl_mip(texture.mip_filter) as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, GLenum::from(texture.wrap_mode) as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, GLenum::from(texture.wrap_mode) as GLint);
        }
    }

    fn scale_framebuffer(&mut self, format: ShaderFormat, viewport: &Viewport, original: &Texture, source: &Texture) -> Size {
        let mut width = 0f32;
        let mut height = 0f32;

        match self.config.scaling.x {
            Scaling {
                scale_type: ScaleType::Input,
                factor
            } => {
                width = source.image.size.width * factor
            },
            Scaling {
                scale_type: ScaleType::Absolute,
                factor
            } => {
                width = factor.into()
            }
            Scaling {
                scale_type: ScaleType::Viewport,
                factor
            } => {
                width = viewport.size.width * factor
            }
        };

        match self.config.scaling.y {
            Scaling {
                scale_type: ScaleType::Input,
                factor
            } => {
                height = source.image.size.height * factor
            },
            Scaling {
                scale_type: ScaleType::Absolute,
                factor
            } => {
                height = factor.into()
            }
            Scaling {
                scale_type: ScaleType::Viewport,
                factor
            } => {
                height = viewport.size.height * factor
            }
        };

        let size = Size {
            width: width.round() as u32,
            height: height.round() as u32
        };

        if self.framebuffer.size != size {
            self.framebuffer.size = size;

            self.framebuffer.init(size,if format == ShaderFormat::Unknown {
                ShaderFormat::R8G8B8A8Unorm
            } else {
                format
            });
        }
        size
    }

    pub fn build_commands(&mut self, parent: &FilterCommon, mvp: Option<&[f32]>, frame_count: u32, frame_direction: u32, viewport: &Viewport, original: &Texture, source: &Texture) {
        let mut fb_format = ShaderFormat::R8G8B8A8Unorm;
        if self.config.srgb_framebuffer {
            fb_format = ShaderFormat::R8G8B8A8Srgb;
        } else if self.config.float_framebuffer {
            fb_format = ShaderFormat::R16G16B16A16Sfloat;
        }

        let fb_size = self.scale_framebuffer(fb_format, viewport, original, source);

        // println!("[frame] Using framebuffer {}, image {}", self.framebuffer.framebuffer, self.framebuffer.image);
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer.framebuffer);
            gl::UseProgram(self.program);
        }

        self.build_semantics(parent, mvp, frame_count, frame_direction, fb_size, viewport, original, source);
        // shader_gl3:1514

        if self.ubo_location.vertex != gl::INVALID_INDEX && self.ubo_location.fragment != gl::INVALID_INDEX {
            if let (Some(ubo), Some(ring)) = (&self.reflection.ubo, &mut self.ubo_ring) {
                let size = ubo.size;
                let buffer = ring.current();

                unsafe {
                    gl::BindBuffer(gl::UNIFORM_BUFFER, *buffer);
                    gl::BufferSubData(gl::UNIFORM_BUFFER, 0, size as GLsizeiptr,
                                      self.uniform_buffer.as_ptr().cast());
                    gl::BindBuffer(gl::UNIFORM_BUFFER, 0);

                    if self.ubo_location.vertex != gl::INVALID_INDEX {
                        gl::BindBufferBase(gl::UNIFORM_BUFFER, self.ubo_location.vertex, *buffer);
                    }
                    if self.ubo_location.vertex != gl::INVALID_INDEX {
                        gl::BindBufferBase(gl::UNIFORM_BUFFER, self.ubo_location.fragment, *buffer);
                    }
                }
                ring.next()
            }
        }

        // todo: final pass?

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer.framebuffer);
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::ClearColor(0.0f32, 0.0f32, 0.0f32, 0.0f32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            //
            gl::Viewport(0, 0, fb_size.width as GLsizei, fb_size.height as GLsizei);

            if self.framebuffer.format == gl::SRGB8_ALPHA8 {
                gl::Enable(gl::FRAMEBUFFER_SRGB);
            } else {
                gl::Disable(gl::FRAMEBUFFER_SRGB);
            }

            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
            gl::Disable(gl::DEPTH_TEST);

            gl::EnableVertexAttribArray(0);
            gl::EnableVertexAttribArray(1);
            gl::BindBuffer(gl::ARRAY_BUFFER, parent.quad_vbo);

            /// the provided pointers are of OpenGL provenance with respect to the buffer bound to quad_vbo,
            /// and not a known provenance to the Rust abstract machine, therefore we give it invalid pointers.
            /// that are inexpressible in Rust
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, (4 * std::mem::size_of::<f32>()) as GLsizei,
                                    std::ptr::invalid(0));
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, (4 * std::mem::size_of::<f32>()) as GLsizei,
                                    std::ptr::invalid(2 * std::mem::size_of::<f32>()));
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);


            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::DisableVertexAttribArray(0);
            gl::DisableVertexAttribArray(1);

            gl::Disable(gl::FRAMEBUFFER_SRGB);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }


        // todo: draw image onto fbo
        // shader_gl3 1579
    }

    // framecount should be pre-modded
    fn build_semantics(&mut self, parent: &FilterCommon, mvp: Option<&[f32]>, frame_count: u32, frame_direction: u32, fb_size: Size, viewport: &Viewport, original: &Texture, source: &Texture) {

        if let Some(variable) = self.reflection.meta.variable_meta.get(&VariableSemantics::MVP) {
            let mvp = mvp.unwrap_or(&[
                2f32, 0.0, 0.0, 0.0,
                0.0, 2.0, 0.0, 0.0,
                0.0, 0.0, 2.0, 0.0,
                -1.0, -1.0, 0.0, 1.0
            ]);

            let mvp_size = mvp.len() * std::mem::size_of::<f32>();
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };
            FilterPass::build_mvp(&mut buffer[offset..][..mvp_size], mvp)
        }

        if let Some(variable) = self.reflection.meta.variable_meta.get(&VariableSemantics::Output) {
            let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };
            FilterPass::build_vec4(location, &mut buffer[offset..][..4], fb_size)
        }

        if let Some(variable) = self.reflection.meta.variable_meta.get(&VariableSemantics::FinalViewport) {
            // todo: do all variables have location..?
            let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };
            FilterPass::build_vec4(location, &mut buffer[offset..][..4], viewport.size)
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


        if let Some(binding) = self.reflection.meta.texture_meta.get(&TextureSemantics::Original.semantics(0)) {
            eprintln!("setting original binding to {}", binding.binding);
            FilterPass::set_texture(binding, original);
        }
        if let Some(variable) = self.reflection.meta.texture_size_meta.get(&TextureSemantics::Original.semantics(0)) {
            let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };
            FilterPass::build_vec4(location, &mut buffer[offset..][..4], original.image.size);

        }


        if let Some(binding) = self.reflection.meta.texture_meta.get(&TextureSemantics::Source.semantics(0)) {
            // eprintln!("setting source binding to {}", binding.binding);
            FilterPass::set_texture(binding, source);
        }
        if let Some(variable) = self.reflection.meta.texture_size_meta.get(&TextureSemantics::Source.semantics(0)) {
            let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };
            FilterPass::build_vec4(location, &mut buffer[offset..][..4], source.image.size);
        }


        // todo: history

        // if let Some(binding) = self.reflection.meta.texture_meta.get(&TextureSemantics::OriginalHistory.semantics(0)) {
        //     FilterPass::set_texture(binding, original);
        // }
        // if let Some(variable) = self.reflection.meta.texture_size_meta.get(&TextureSemantics::OriginalHistory.semantics(0)) {
        //     let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
        //     let (buffer, offset) = match variable.offset {
        //         MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
        //         MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
        //     };
        //     FilterPass::build_vec4(location, &mut buffer[offset..][..4], original.image.size);
        // }

        for variable in self.reflection.meta.parameter_meta.values() {
            let location = self.locations.get(&variable.id).expect("variable did not have location mapped").location();
            let (buffer, offset) = match variable.offset {
                MemberOffset::Ubo(offset) => (&mut self.uniform_buffer, offset),
                MemberOffset::PushConstant(offset) => (&mut self.push_buffer, offset)
            };

            // presets override params
            let default = self.source.parameters.iter().find(|&p| p.id == variable.id)
                .map(|f| f.initial)
                .unwrap_or(0f32);

            let value = parent.preset.parameters.iter().find(|&p| p.name == variable.id)
                .map(|p| p.value)
                .unwrap_or(default);

            FilterPass::build_float(location, &mut buffer[offset..][..4], value)
        }

        // todo: deal with both lut name and index
        // for (index, lut) in parent.luts.values().enumerate() {
        //     // todo: sort out order
        //     if let Some(variable) = self.reflection.meta.texture_size_meta.get(&TextureSemantics::User.semantics(index as u32)) {
        //     }
        //
        // }
        // todo history
    }
}