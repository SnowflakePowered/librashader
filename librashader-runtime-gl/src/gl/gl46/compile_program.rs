use glow::HasContext;
use crate::binding::UniformLocation;
use crate::error::FilterChainError;
use crate::gl::CompileProgram;
use crate::util;
use gl::types::{GLint, GLsizei, GLuint};
use librashader_cache::Cacheable;
use librashader_reflect::back::glsl::CrossGlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use spirv_cross::spirv::Decoration;

pub struct Gl4CompileProgram;

struct GlProgramBinary {
    program: Vec<u8>,
    format: u32,
}

impl Cacheable for GlProgramBinary {
    fn from_bytes(cached: &[u8]) -> Option<Self>
    where
        Self: Sized,
    {
        let mut cached = Vec::from(cached);
        let format = cached.split_off(cached.len() - std::mem::size_of::<u32>());
        let format: Option<&u32> = bytemuck::try_from_bytes(&format).ok();
        let Some(format) = format else {
            return None;
        };

        return Some(GlProgramBinary {
            program: cached,
            format: *format,
        });
    }

    fn to_bytes(&self) -> Option<Vec<u8>> {
        let mut slice = self.program.clone();
        slice.extend(bytemuck::bytes_of(&self.format));
        Some(slice)
    }
}

impl CompileProgram for Gl4CompileProgram {
    fn compile_program(
        context: &glow::Context,
        glsl: ShaderCompilerOutput<String, CrossGlslContext>,
        cache: bool,
    ) -> crate::error::Result<(glow::Program, UniformLocation<Option<glow::UniformLocation>>)> {
        let vertex_resources = glsl.context.artifact.vertex.get_shader_resources()?;

        let program = librashader_cache::cache_shader_object(
            "opengl4",
            &[glsl.vertex.as_str(), glsl.fragment.as_str()],
            |&[vertex, fragment]| unsafe {
                let vertex = util::gl_compile_shader(context, glow::VERTEX_SHADER, vertex)?;
                let fragment = util::gl_compile_shader(context, glow::FRAGMENT_SHADER, fragment)?;

                let program = context.create_program()
                    .map_err(|_| FilterChainError::GlProgramError)?;

                context.attach_shader(program, vertex);
                context.attach_shader(program, fragment);

                for res in &vertex_resources.stage_inputs {
                    let loc = glsl
                        .context
                        .artifact
                        .vertex
                        .get_decoration(res.id, Decoration::Location)?;

                    context.bind_attrib_location(program, loc, &res.name);
                }
                context.link_program(program);
                context.delete_shader(vertex);
                context.delete_shader(fragment);

                if !context.get_program_link_status(program) {
                    return Err(FilterChainError::GLLinkError);
                }

                context.get_program_binary
                let length = context.get_program_resource_i32(program, )

                let mut length = 0;
                gl::GetProgramiv(program, gl::PROGRAM_BINARY_LENGTH, &mut length);

                let mut binary = vec![0; length as usize];
                let mut format = 0;
                gl::GetProgramBinary(
                    program,
                    length,
                    std::ptr::null_mut(),
                    &mut format,
                    binary.as_mut_ptr().cast(),
                );
                gl::DeleteProgram(program);
                Ok(GlProgramBinary {
                    program: binary,
                    format,
                })
            },
            |GlProgramBinary {
                 program: blob,
                 format,
             }| {
                let program = unsafe {
                    let program = gl::CreateProgram();
                    gl::ProgramBinary(program, format, blob.as_ptr().cast(), blob.len() as GLsizei);
                    program
                };

                unsafe {
                    let mut status = 0;
                    gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
                    if status != 1 {
                        return Err(FilterChainError::GLLinkError);
                    }

                    if gl::GetError() == gl::INVALID_ENUM {
                        return Err(FilterChainError::GLLinkError);
                    }
                }
                return Ok(program);
            },
            !cache,
        )?;

        let ubo_location = unsafe {
            for (name, binding) in &glsl.context.sampler_bindings {
                let location = gl::GetUniformLocation(program, name.as_str().as_ptr().cast());
                if location >= 0 {
                    gl::ProgramUniform1i(program, location, *binding as GLint);
                }
            }

            UniformLocation {
                vertex: gl::GetUniformBlockIndex(program, b"LIBRA_UBO_VERTEX\0".as_ptr().cast()),
                fragment: gl::GetUniformBlockIndex(
                    program,
                    b"LIBRA_UBO_FRAGMENT\0".as_ptr().cast(),
                ),
            }
        };

        Ok((program, ubo_location))
    }
}
