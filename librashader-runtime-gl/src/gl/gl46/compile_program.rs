use glow::{HasContext, NativeUniformLocation};
use crate::binding::UniformLocation;
use crate::error::FilterChainError;
use crate::gl::CompileProgram;
use crate::util;
use gl::types::{GLint, GLsizei, GLuint};
use librashader_cache::Cacheable;
use librashader_reflect::back::glsl::CrossGlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use spirv_cross::spirv::{Decoration, ShaderResources};

pub struct Gl4CompileProgram;

struct GlProgramBinary(Option<glow::ProgramBinary>);

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

        return Some(GlProgramBinary(Some(glow::ProgramBinary {
            buffer: cached,
            format: *format,
        })));
    }

    fn to_bytes(&self) -> Option<Vec<u8>> {
        let Some(binary) = self.0 else {
            None
        };

        let mut slice = binary.buffer.clone();
        slice.extend(bytemuck::bytes_of(&binary.format));
        Some(slice)
    }
}

impl CompileProgram for Gl4CompileProgram {
    fn compile_program(
        context: &glow::Context,
        glsl: ShaderCompilerOutput<String, CrossGlslContext>,
        cache: bool,
    ) -> crate::error::Result<(glow::Program, UniformLocation<Option<glow::UniformLocation>>)> {
        unsafe fn compile_shader(context: &glow::Context, resources: &CrossGlslContext, vertex: &str, fragment: &str) -> crate::error::Result<glow::Program> {
            let vertex_resources = resources.artifact.vertex.get_shader_resources()?;;
            let vertex = util::gl_compile_shader(context, glow::VERTEX_SHADER, vertex)?;
            let fragment = util::gl_compile_shader(context, glow::FRAGMENT_SHADER, fragment)?;

            let program = context.create_program()
                .map_err(|_| FilterChainError::GlProgramError)?;

            context.attach_shader(program, vertex);
            context.attach_shader(program, fragment);

            for res in &vertex_resources.stage_inputs {
                let loc = resources
                    .artifact
                    .vertex
                    .get_decoration(res.id, Decoration::Location)?;

                context.bind_attrib_location(program, loc, &res.name);
            }
            context.program_binary_retrievable_hint(program, true);
            context.link_program(program);
            context.delete_shader(vertex);
            context.delete_shader(fragment);

            if !context.get_program_link_status(program) {
                return Err(FilterChainError::GLLinkError);
            }
            Ok(program)
        }

        let program = librashader_cache::cache_shader_object(
            "opengl4",
            &[glsl.vertex.as_str(), glsl.fragment.as_str()],
            |&[vertex, fragment]| unsafe {
                let program = compile_shader(context, &glsl.context, vertex, fragment)?;
                let program_binary = context.get_program_binary(program);
                context.delete_program(program);
                Ok(GlProgramBinary(program_binary))
            },
            |binary| unsafe {
                let program = context.create_program()
                    .map_err(|_| FilterChainError::GlProgramError)?;

                if let Some(binary) = &binary.0 {
                    context.program_binary(program, binary);
                }

                if !context.get_program_link_status(program) {
                    context.delete_program(program);
                    return compile_shader(context, &glsl.context, glsl.vertex.as_str(), glsl.fragment.as_str())
                }

                return Ok(program);
            },
            !cache,
        )?;

        let ubo_location = unsafe {
            for (name, binding) in &glsl.context.sampler_bindings {
                let location = context.get_uniform_location(program, name.as_str());
                if let Some(location) = location {
                    gl::ProgramUniform1i(program, location, *binding as GLint);
                }
            }

            UniformLocation {
                vertex: context.get_uniform_block_index(program, "LIBRA_UBO_VERTEX")
                    .and_then(|v| glow::UniformLocation::from(NativeUniformLocation(v))),
                fragment: gl::GetUniformBlockIndex(
                    program,
                    b"LIBRA_UBO_FRAGMENT\0".as_ptr().cast(),
                ),
            }
        };

        Ok((program, ubo_location))
    }
}
