mod hello_triangle;
mod filter;
mod filter_pass;
mod util;
mod framebuffer;

use std::collections::HashMap;
use std::error::Error;
use std::iter::Filter;
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
use librashader_reflect::reflect::semantics::{MemberOffset, SemanticMap, TextureSemantics, VariableMeta, VariableSemantics};
use librashader_reflect::reflect::{TextureSemanticMap, VariableSemanticMap};
use util::{Location, VariableLocation, RingBuffer, Size, GlImage, Texture, Viewport};

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

impl FilterChain {
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

    fn reflect_parameter(pipeline: GLuint, meta: &VariableMeta) -> VariableLocation {
        // todo: support both ubo and pushco
        // todo: fix this.
        match meta.offset {
            MemberOffset::Ubo(_) => {
                let vert_name = format!("RARCH_UBO_VERTEX_INSTANCE.{}\0", meta.id);
                let frag_name = format!("RARCH_UBO_FRAGMENT_INSTANCE.{}\0", meta.id);
                unsafe {
                    let vertex = gl::GetUniformLocation(pipeline, vert_name.as_ptr().cast());
                    let fragment = gl::GetUniformLocation(pipeline, frag_name.as_ptr().cast());

                    VariableLocation::Ubo(Location {
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

                    VariableLocation::Push(Location {
                        vertex,
                        fragment
                    })
                }
            }
        }
    }

}

pub struct FilterChain {
    passes: Vec<FilterPass>,
    semantics: ReflectSemantics,
    preset: ShaderPreset,
    original_history: Vec<Framebuffer>,
    history: Vec<Texture>,
    feedback: Vec<Texture>,
    luts: FxHashMap<String, Texture>
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
            FilterChain::load_pass_semantics(&mut uniform_semantics, &mut texture_semantics, details.0)
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
                    gl::GenBuffers(16, ring.items_mut().as_mut_ptr());
                    for buffer in ring.items() {
                        gl::BindBuffer(gl::UNIFORM_BUFFER, *buffer);
                        gl::BufferData(gl::UNIFORM_BUFFER, size as GLsizeiptr, std::ptr::null(), gl::STREAM_DRAW);
                    }
                    gl::BindBuffer(gl::UNIFORM_BUFFER, 0);
                }
                Some(ring)
            } else {
                None
            };

            let uniform_buffer = vec![0; reflection.ubo.as_ref().map(|ubo| ubo.size as usize).unwrap_or(0)].into_boxed_slice();
            let push_buffer = vec![0; reflection.push_constant.as_ref().map(|push| push.size as usize).unwrap_or(0)].into_boxed_slice();

            // todo: reflect indexed parameters
            let mut locations = FxHashMap::default();
            for param in reflection.meta.parameter_meta.values() {
                locations.insert(param.id.clone(), FilterChain::reflect_parameter(program, param));
            }

            for param in reflection.meta.variable_meta.values() {
                locations.insert(param.id.clone(), FilterChain::reflect_parameter(program, param));
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
                source,
                // no idea if this works.
                // retroarch checks if feedback frames are used but we'll just init it tbh.
                framebuffer: Framebuffer::new(1),
                feedback_framebuffer: Framebuffer::new(1),
                config: config.clone()
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


        // load luts
        let mut luts = FxHashMap::default();

        for texture in &preset.textures {
            let image = Image::load(&texture.path)?;
            let levels = if texture.mipmap {
                util::calc_miplevel(image.width, image.height)
            } else {
                1u32
            };

            let mut handle = 0;
            unsafe {
                gl::GenTextures(1, &mut handle);
                gl::BindTexture(gl::TEXTURE_2D, handle);
                gl::TexStorage2D(gl::TEXTURE_2D, levels as GLsizei, gl::RGBA8,
                                 image.width as GLsizei, image.height as GLsizei);

                gl::PixelStorei(gl::UNPACK_ROW_LENGTH, 0);
                gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);
                gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0);
                gl::TexSubImage2D(gl::TEXTURE_2D, 0, 0, 0,
                                  image.width as GLsizei, image.height as GLsizei,
                                  gl::RGBA, gl::UNSIGNED_BYTE, image.bytes.as_ptr().cast());

                let mipmap = levels > 1;
                let linear = texture.filter_mode == FilterMode::Linear;

                // set mipmaps and wrapping

                if mipmap {
                    gl::GenerateMipmap(gl::TEXTURE_2D);
                }

                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, GLenum::from(texture.wrap_mode) as GLint);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, GLenum::from(texture.wrap_mode) as GLint);

                if !linear {
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
                } else {
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
                    if mipmap {
                        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER,
                                          gl::LINEAR_MIPMAP_LINEAR as GLint);
                    } else {
                        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER,
                                          gl::LINEAR as GLint);
                    }
                }

                gl::BindTexture(gl::TEXTURE_2D, 0);
            }

            luts.insert(texture.name.clone(), Texture {
                image: GlImage {
                    handle,
                    format: gl::RGBA8,
                    size: Size {
                        width: image.width,
                        height: image.height
                    },
                    padded_size: Size::default()
                },
                filter: texture.filter_mode,
                mip_filter: texture.filter_mode,
                wrap_mode: texture.wrap_mode
            });
        }

        // todo: split params
        Ok(FilterChain {
            passes: filters,
            semantics,
            preset,
            original_history: vec![],
            history: vec![],
            feedback: vec![],
            luts,
        })
    }


    // how much info do we actually need?
    fn frame(&mut self, count: u32, vp: &Viewport, input: GlImage, clear: bool) {

        let filter = self.preset.shaders.first().map(|f| f.filter).unwrap_or_default();
        let wrap_mode = self.preset.shaders.first().map(|f| f.wrap_mode).unwrap_or_default();
        let original = Texture {
            image: input,
            filter,
            mip_filter: filter,
            wrap_mode
        };

        let mut source = original.clone();

        for passes in &mut self.passes {
            // passes.build_semantics(&self, None, count, 1, vp, &original, &source);
        }

        // todo: deal with the mess that is frame history
    }

    pub fn do_final_pass(&mut self, count: u64, vp: &Viewport, input: GlImage, clear: bool, mvp: &[f32]) {

        // todo: make copy

        // todo: get filter info from pass data.
        let filter = self.preset.shaders.first().map(|f| f.filter).unwrap_or_default();
        let wrap_mode = self.preset.shaders.first().map(|f| f.wrap_mode).unwrap_or_default();
        let original = Texture {
            image: input,
            filter,
            mip_filter: filter,
            wrap_mode
        };





        // todo: deal with the mess that is frame history
    }
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
