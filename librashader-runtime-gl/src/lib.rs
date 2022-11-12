mod hello_triangle;

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use gl::types::{GLenum, GLint, GLsizeiptr, GLuint};
use rustc_hash::FxHashMap;
use spirv_cross::spirv::Decoration;

use librashader::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::{CompileShader, ShaderCompilerOutput};
use librashader_reflect::back::cross::{GlslangGlslContext, GlVersion};
use librashader_reflect::back::targets::{FromCompilation, GLSL};
use librashader_reflect::front::shaderc::GlslangCompilation;
use librashader_reflect::reflect::cross::CrossReflect;
use librashader_reflect::reflect::{ReflectSemantics, ReflectShader, ShaderReflection, UniformSemantic};
use librashader_reflect::reflect::semantics::{MemberOffset, SemanticMap, TextureSemantics, VariableMeta, VariableSemantics};
use librashader_reflect::reflect::{TextureSemanticMap, VariableSemanticMap};

pub fn load_pass_semantics(uniform_semantics: &mut FxHashMap<String, UniformSemantic>, texture_semantics: &mut FxHashMap<String, SemanticMap<TextureSemantics>>,
                           config: &ShaderPassConfig) {
    let Some(alias) = &config.alias else {
        return;
    };

    // Ignore empty aliases
    if alias.trim().is_empty() {
        return;
    }

    let index = config.id as u32;

    // PassOutput
    texture_semantics.insert(alias.clone(), SemanticMap {
        semantics: TextureSemantics::PassOutput,
        index
    });
    uniform_semantics.insert(format!("{alias}Size"), UniformSemantic::Texture(SemanticMap {
        semantics: TextureSemantics::PassOutput,
        index
    }));

    // PassFeedback
    texture_semantics.insert(format!("{alias}Feedback"), SemanticMap {
        semantics: TextureSemantics::PassFeedback,
        index
    });
    uniform_semantics.insert(format!("{alias}FeedbackSize"), UniformSemantic::Texture(SemanticMap {
        semantics: TextureSemantics::PassFeedback,
        index
    }));

}

pub struct RingBuffer<T, const SIZE: usize> {
    items: [T; SIZE],
    index: usize
}

impl <T, const SIZE: usize> RingBuffer<T, SIZE>
where T: Copy, T: Default
{
    pub fn new() -> Self {
        Self {
            items: [T::default(); SIZE],
            index: 0
        }
    }
}

impl <T, const SIZE: usize> RingBuffer<T, SIZE> {
    pub fn current(&self) -> &T {
        &self.items[self.index]
    }

    pub fn next(&mut self) {
        self.index += 1;
        if self.index >= SIZE {
            self.index = 0
        }
    }
}


#[derive(Debug)]
pub struct Location<T> {
    vertex: T,
    fragment: T,
}

#[derive(Debug)]
pub enum ParameterLocation {
    Ubo(Location<GLint>),
    Push(Location<GLint>),
}
pub struct FilterPass {
    reflection: ShaderReflection,
    compiled: ShaderCompilerOutput<String, GlslangGlslContext>,
    program: GLuint,
    ubo_location: Location<GLuint>,
    ubo_ring: Option<RingBuffer<GLuint, 16>>,
    uniform_buffer: Vec<u8>,
    push_buffer: Vec<u8>,
    locations: FxHashMap<String, ParameterLocation>
}

pub struct FilterChain {
    reflections: Vec<ShaderReflection>,
    compiled: Vec<ShaderCompilerOutput<String, GlslangGlslContext>>,
    programs: Vec<GLuint>,
    ubo_location: Location<GLint>,
}

pub fn reflect_parameter(pipeline: GLuint, meta: &VariableMeta) -> ParameterLocation {
    // todo: support both ubo and pushco
    // todo: fix this.
    match meta.offset {
        MemberOffset::Ubo(_) => {
            let vert_name = format!("RARCH_UBO_VERTEX_INSTANCE.{}\0", meta.id);
            let frag_name = format!("RARCH_UBO_FRAGMENT_INSTANCE.{}\0", meta.id);
            unsafe {
                let vertex = gl::GetUniformLocation(pipeline, vert_name.as_ptr().cast());
                let fragment = gl::GetUniformLocation(pipeline, frag_name.as_ptr().cast());

                ParameterLocation::Ubo(Location {
                    vertex,
                    fragment
                })
            }
        }
        MemberOffset::PushConstant(_) => {
            let vert_name = format!("RARCH_PUSH_VERTEX_INSTANCE.{}\0", meta.id);
            let frag_name = format!("RARCH_PUSH_FRAGMENT_INSTANCE.{}\0", meta.id);
            unsafe {
                let vertex = gl::GetUniformLocation(pipeline, vert_name.as_ptr().cast());
                let fragment = gl::GetUniformLocation(pipeline, frag_name.as_ptr().cast());

                ParameterLocation::Push(Location {
                    vertex,
                    fragment
                })
            }
        }
    }
}
// todo: init gl

pub fn load(path: impl AsRef<Path>) -> Result<(), Box<dyn Error>>{
    let preset = librashader_presets::ShaderPreset::try_parse(path)?;
    let mut passes: Vec<(&ShaderPassConfig, ShaderSource, _)> = preset.shaders.iter()
        .map(|shader| {
            let source = librashader_preprocess::load_shader_source(&shader.name)
                .unwrap();
            let spirv = librashader_reflect::front::shaderc::compile_spirv(&source)
                .unwrap();
            let mut reflect = GLSL::from_compilation(spirv).unwrap();
            (shader, source, reflect)
        }).collect();

    // todo: this can probably be extracted out.
    let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();
    let mut texture_semantics: FxHashMap<String, SemanticMap<TextureSemantics>> = Default::default();

    for details in &passes {
        load_pass_semantics(&mut uniform_semantics, &mut texture_semantics, details.0)
    }

    // add float params
    for (index, parameter) in preset.parameters.iter().enumerate() {
        uniform_semantics.insert(parameter.name.clone(), UniformSemantic::Variable(SemanticMap {
            semantics: VariableSemantics::FloatParameter,
            index: index as u32
        }));
    }

    // add lut params
    for (index, texture) in preset.textures.iter().enumerate() {
        texture_semantics.insert(texture.name.clone(), SemanticMap {
            semantics: TextureSemantics::User,
            index: index as u32
        });
    }

    let semantics = ReflectSemantics {
        uniform_semantics,
        non_uniform_semantics: texture_semantics
    };

    let mut filters = Vec::new();

    for (index, (config, source, mut reflect)) in passes.into_iter().enumerate() {
        let reflection = reflect.reflect(index as u32, &semantics)?;
        let glsl = reflect.compile(GlVersion::V4_60)?;

        let vertex_resources = glsl.context.compiler.vertex.get_shader_resources()?;

        // todo: split this out.
        let (program, ubo_location) = unsafe {
            let vertex = gl_compile_shader(gl::VERTEX_SHADER, glsl.vertex.as_str());
            let fragment = gl_compile_shader(gl::FRAGMENT_SHADER, glsl.fragment.as_str());

            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex);
            gl::AttachShader(program, fragment);

            for res in &vertex_resources.stage_inputs {
                let loc = glsl.context.compiler.vertex.get_decoration(res.id, Decoration::Location)?;
                let loc_name = format!("RARCH_ATTRIBUTE_{loc}");
                gl::BindAttribLocation(program, loc, loc_name.as_str().as_ptr().cast())
            }
            gl::LinkProgram(program);
            gl::DeleteShader(vertex);
            gl::DeleteShader(fragment);

            let mut status = 0;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
            if status != 1 {
                panic!("failed to link program")
            }

            for binding in &glsl.context.texture_fixups {
                let loc_name = format!("RARCH_TEXTURE_{}", *binding);
                unsafe {
                    let location = gl::GetUniformLocation(program, loc_name.as_str().as_ptr().cast());
                    if location >= 0 {
                        gl::Uniform1i(location, *binding as GLint);
                    }
                }
            }

            unsafe {
                gl::UseProgram(0);
                (program, Location {
                    vertex: gl::GetUniformBlockIndex(program, b"RARCH_UBO_VERTEX\0".as_ptr().cast()),
                    fragment:  gl::GetUniformBlockIndex(program, b"RARCH_UBO_FRAGMENT\0".as_ptr().cast()),
                })
            }
        };

        let ubo_ring = if let Some(ubo) = &reflection.ubo {
            let size = ubo.size;
            let mut ring: RingBuffer<GLuint, 16> = RingBuffer::new();
            unsafe {
                gl::GenBuffers(16, ring.items.as_mut_ptr());
                for buffer in &ring.items {
                    gl::BindBuffer(gl::UNIFORM_BUFFER, *buffer);
                    gl::BufferData(gl::UNIFORM_BUFFER, size as GLsizeiptr, std::ptr::null(), gl::STREAM_DRAW);
                }
                gl::BindBuffer(gl::UNIFORM_BUFFER, 0);
            }
            Some(ring)
        } else {
            None
        };

        let uniform_buffer = vec![0; reflection.ubo.as_ref().map(|ubo| ubo.size as usize).unwrap_or(0)];
        let push_buffer =  vec![0; reflection.push_constant.as_ref().map(|push| push.size as usize).unwrap_or(0)];

        // todo: reflect indexed parameters
        let mut locations = FxHashMap::default();
        for param in reflection.meta.parameter_meta.values() {
            locations.insert(param.id.clone(), reflect_parameter(program, param));
        }

        for param in reflection.meta.variable_meta.values() {
            locations.insert(param.id.clone(), reflect_parameter(program, param));
        }


        eprintln!("{:#?}", semantics);
        eprintln!("{:#?}", reflection.meta);
        eprintln!("{:#?}", locations);
        eprintln!("{:#?}", reflection.push_constant);
        eprintln!("====fragment====");
        eprintln!("{:#}", glsl.fragment);
        eprintln!("====vertex====");
        eprintln!("{:#}", glsl.vertex);

        filters.push(FilterPass {
            reflection,
            compiled: glsl,
            program,
            ubo_location,
            ubo_ring,
            uniform_buffer,
            push_buffer,
            locations
        });
    }

    // let mut glprogram: Vec<GLuint> = Vec::new();
    // for compilation in &compiled {
    //     // compilation.context.compiler.vertex
    // }

//    eprintln!("{:#?}", reflections);

    // eprintln!("{:#?}", compiled./);
    // eprintln!("{:?}", preset);
    // eprintln!("{:?}", reflect.reflect(&ReflectOptions {
    //     pass_number: i as u32,
    //     uniform_semantics,
    //     non_uniform_semantics: Default::default(),
    // }));

    Ok(())
}

unsafe fn gl_compile_shader(stage: GLenum, source: &str) -> GLuint {
    let shader = gl::CreateShader(stage);
    gl::ShaderSource(shader, 1, &source.as_bytes().as_ptr().cast(), std::ptr::null());
    gl::CompileShader(shader);

    let mut compile_status = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut compile_status);

    if compile_status == 0 {
        panic!("failed to compile")
    }
    shader
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle() {
        let (glfw, window, events, shader, vao) = hello_triangle::setup();
            load("../test/basic.slangp")
                .unwrap();
        hello_triangle::do_loop(glfw, window, events, shader, vao);
    }

    // #[test]
    // fn load_preset() {
    //
    //     load("../test/basic.slangp")
    //         .unwrap();
    // }
}
