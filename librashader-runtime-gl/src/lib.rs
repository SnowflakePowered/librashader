mod hello_triangle;
mod filter;

use std::collections::HashMap;
use std::error::Error;
use std::iter::Filter;
use std::path::Path;
use gl::types::{GLenum, GLint, GLsizei, GLsizeiptr, GLuint};
use glfw::Key::P;
use rustc_hash::FxHashMap;
use spirv_cross::spirv::Decoration;

use librashader::{ShaderFormat, ShaderSource};
use librashader_presets::{ShaderPassConfig, ShaderPreset};
use librashader_reflect::back::{CompileShader, ShaderCompilerOutput};
use librashader_reflect::back::cross::{GlslangGlslContext, GlVersion};
use librashader_reflect::back::targets::{FromCompilation, GLSL};
use librashader_reflect::front::shaderc::GlslangCompilation;
use librashader_reflect::reflect::cross::CrossReflect;
use librashader_reflect::reflect::{ReflectSemantics, ReflectShader, ShaderReflection, UniformSemantic};
use librashader_reflect::reflect::semantics::{MemberOffset, SemanticMap, TextureSemantics, VariableMeta, VariableSemantics};
use librashader_reflect::reflect::{TextureSemanticMap, VariableSemanticMap};

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

fn load_pass_semantics(uniform_semantics: &mut FxHashMap<String, UniformSemantic>, texture_semantics: &mut FxHashMap<String, SemanticMap<TextureSemantics>>,
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
    locations: FxHashMap<String, ParameterLocation>,
    framebuffer: Framebuffer,
    feedback_framebuffer: Framebuffer,
}

pub struct Framebuffer {
    image: GLuint,
    size: Size,
    format: GLenum,
    max_levels: u32,
    levels: u32,
    framebuffer: GLuint,
    init: bool
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if self.framebuffer != 0 {
            unsafe {
                gl::DeleteFramebuffers(1, &self.framebuffer);
            }
        }

        if self.image != 0 {
            unsafe {
                gl::DeleteTextures(1, &self.image);
            }
        }
    }
}
impl Framebuffer {
    pub fn new(max_levels: u32) -> Framebuffer {
        let mut framebuffer = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
        }

        Framebuffer {
            image: 0,
            size: Size { width: 1, height: 1 },
            format: 0,
            max_levels,
            levels: 0,
            framebuffer,
            init: false
        }
    }

    fn init(&mut self, mut size: Size, mut format: ShaderFormat) {
        if format == ShaderFormat::Unknown {
            format = ShaderFormat::R8G8B8A8Unorm;
        }

        self.format = GLenum::from(format);
        self.size = size;

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer);

            // reset the framebuffer image
            if self.image != 0 {
                gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, 0, 0);
                gl::DeleteTextures(1, &self.image);
            }

            gl::GenTextures(1, &mut self.image);
            gl::BindTexture(1, self.image);

            if size.width == 0 {
                size.width = 1;
            }
            if size.height == 0 {
                size.height = 1;
            }

            self.levels = calc_miplevel(size.width, size.height);
            if self.levels > self.max_levels {
                self.levels = self.max_levels;
            }
            if self.levels == 0 {
                self.levels = 1;
            }

            gl::TexStorage2D(gl::TEXTURE_2D, self.levels as GLsizei, self.format, size.width as GLsizei, size.height as GLsizei);
            gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                   gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, self.image, 0);

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                match status {
                    gl::FRAMEBUFFER_UNSUPPORTED => {
                        eprintln!("unsupported fbo");

                        gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                                 gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, 0, 0);
                        gl::DeleteTextures(1, &self.image);
                        gl::GenTextures(1, &mut self.image);
                        gl::BindTexture(1, self.image);

                        self.levels = calc_miplevel(size.width, size.height);
                        if self.levels > self.max_levels {
                            self.levels = self.max_levels;
                        }
                        if self.levels == 0 {
                            self.levels = 1;
                        }

                        gl::TexStorage2D(gl::TEXTURE_2D, self.levels as GLsizei, gl::RGBA8, size.width as GLsizei, size.height as GLsizei);
                        gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                                 gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, self.image, 0);
                        self.init = gl::CheckFramebufferStatus(gl::FRAMEBUFFER) == gl::FRAMEBUFFER_COMPLETE;
                    }
                    _ => panic!("failed to complete: {status}")
                }
            } else {
                self.init = true;
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

pub fn calc_miplevel(width: u32, height: u32) -> u32 {
    let mut size = std::cmp::max(width, height);
    let mut levels = 0;
    while size != 0 {
        levels += 1;
        size >>= 1;
    }

    return levels;
}
pub struct FilterChain {
    passes: Vec<FilterPass>,
    semantics: ReflectSemantics,
    preset: ShaderPreset,
    original_history: Vec<Framebuffer>,
    history: Vec<Texture>,
    feedback: Vec<Texture>
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

impl FilterChain {
    pub fn load(path: impl AsRef<Path>) -> Result<FilterChain, Box<dyn Error>> {
        let preset = librashader_presets::ShaderPreset::try_parse(path)?;
        let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();
        let mut texture_semantics: FxHashMap<String, SemanticMap<TextureSemantics>> = Default::default();

        let mut passes: Vec<(&ShaderPassConfig, ShaderSource, _)> = preset.shaders.iter()
            .map(|shader| {
                eprintln!("[gl] loading {}", &shader.name.display());
                let source: ShaderSource = librashader_preprocess::load_shader_source(&shader.name)
                    .unwrap();

                let spirv = librashader_reflect::front::shaderc::compile_spirv(&source)
                    .unwrap();
                let mut reflect = GLSL::from_compilation(spirv).unwrap();

                for parameter in source.parameters.iter() {
                    uniform_semantics.insert(parameter.id.clone(), UniformSemantic::Variable(SemanticMap {
                        semantics: VariableSemantics::FloatParameter,
                        index: ()
                    }));
                }

                (shader, source, reflect)
            }).collect();

        // todo: this can probably be extracted out.

        for details in &passes {
            load_pass_semantics(&mut uniform_semantics, &mut texture_semantics, details.0)
        }

        // add lut params
        for (index, texture) in preset.textures.iter().enumerate() {
            texture_semantics.insert(texture.name.clone(), SemanticMap {
                semantics: TextureSemantics::User,
                index: index as u32
            });

            uniform_semantics.insert(format!("{}Size", texture.name), UniformSemantic::Texture(SemanticMap {
                semantics: TextureSemantics::User,
                index: index as u32
            }));
        }

        let semantics = ReflectSemantics {
            uniform_semantics,
            non_uniform_semantics: texture_semantics
        };

        let mut filters = Vec::new();

        // initialize passes
        for (index, (config, source, mut reflect)) in passes.into_iter().enumerate() {
            let mut semantics = semantics.clone();

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
                        fragment: gl::GetUniformBlockIndex(program, b"RARCH_UBO_FRAGMENT\0".as_ptr().cast()),
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
            let push_buffer = vec![0; reflection.push_constant.as_ref().map(|push| push.size as usize).unwrap_or(0)];

            // todo: reflect indexed parameters
            let mut locations = FxHashMap::default();
            for param in reflection.meta.parameter_meta.values() {
                locations.insert(param.id.clone(), reflect_parameter(program, param));
            }

            for param in reflection.meta.variable_meta.values() {
                locations.insert(param.id.clone(), reflect_parameter(program, param));
            }


            // eprintln!("{:#?}", semantics);
            eprintln!("{:#?}", reflection.meta);
            eprintln!("{:#?}", locations);
            eprintln!("{:#?}", reflection.push_constant);
            // eprintln!("====fragment====");
            // eprintln!("{:#}", glsl.fragment);
            // eprintln!("====vertex====");
            // eprintln!("{:#}", glsl.vertex);

            filters.push(FilterPass {
                reflection,
                compiled: glsl,
                program,
                ubo_location,
                ubo_ring,
                uniform_buffer,
                push_buffer,
                locations,

                // no idea if this works.
                // retroarch checks if feedback frames are used but we'll just init it tbh.
                framebuffer: Framebuffer::new(1),
                feedback_framebuffer: Framebuffer::new(1)
            });
        }

        eprintln!("{:?}", filters.iter().map(|f| f.program).collect::<Vec<_>>());
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

        // todo: apply shader pass
        // gl3.cpp: 1942


        Ok(FilterChain {
            passes: filters,
            semantics,
            preset,
            original_history: vec![],
            history: vec![],
            feedback: vec![]
        })
    }


    // how much info do we actually need?
    fn frame(&mut self, count: u64, vp: &Viewport, input: &Texture, clear: bool) {

        // todo: deal with the mess that is frame history
    }
}

#[derive(Debug, Copy, Clone)]
struct Viewport {
    x: i32,
    y: i32,
    width: i32,
    height: i32
}

#[derive(Debug, Copy, Clone)]
struct Size {
    width: u32,
    height: u32,
}

#[derive(Debug, Copy, Clone)]
struct Texture {
    handle: GLuint,
    format: GLenum,
    size: Size,
    padded_size: Size
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle() {
        let (glfw, window, events, shader, vao) = hello_triangle::setup();
        FilterChain::load("../test/basic.slangp").unwrap();
        // FilterChain::load("../test/slang-shaders/crt/crt-royale.slangp").unwrap();

        hello_triangle::do_loop(glfw, window, events, shader, vao);
    }

    // #[test]
    // fn load_preset() {
    //
    //     load("../test/basic.slangp")
    //         .unwrap();
    // }
}
