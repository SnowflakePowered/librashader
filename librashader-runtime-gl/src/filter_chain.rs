use rustc_hash::FxHashMap;
use gl::types::{GLenum, GLint, GLsizei, GLsizeiptr, GLuint};
use librashader_presets::{ShaderPassConfig, ShaderPreset};
use librashader_reflect::reflect::semantics::{MemberOffset, SemanticMap, TextureSemantics, UniformMeta, VariableSemantics};
use librashader_reflect::reflect::{ReflectSemantics, ReflectShader, UniformSemantic};
use std::path::Path;
use std::error::Error;
use librashader::{FilterMode, ShaderSource};
use librashader_reflect::back::cross::GlVersion;
use librashader_reflect::back::targets::{FromCompilation, GLSL};
use spirv_cross::spirv::Decoration;
use librashader::image::Image;
use librashader_reflect::back::CompileShader;
use crate::binding::{UniformBinding, UniformLocation, VariableLocation};
use crate::filter_pass::FilterPass;
use crate::framebuffer::Framebuffer;
use crate::render_target::RenderTarget;
use crate::util;
use crate::util::{GlImage, RingBuffer, Size, Texture, Viewport};

static QUAD_VBO_DATA: &'static [f32; 16] = &[
    0.0f32, 0.0f32, 0.0f32, 0.0f32,
    1.0f32, 0.0f32, 1.0f32, 0.0f32,
    0.0f32, 1.0f32, 0.0f32, 1.0f32,
    1.0f32, 1.0f32, 1.0f32, 1.0f32,
];

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

        let index = config.id as usize;

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

    fn reflect_uniform_location(pipeline: GLuint, meta: &impl UniformMeta) -> VariableLocation {
        // todo: support both ubo and pushco
        // todo: fix this.
        match meta.offset() {
            MemberOffset::Ubo(_) => {
                let vert_name = format!("LIBRA_UBO_VERTEX_INSTANCE.{}\0", meta.id());
                let frag_name = format!("LIBRA_UBO_FRAGMENT_INSTANCE.{}\0", meta.id());
                unsafe {
                    let vertex = gl::GetUniformLocation(pipeline, vert_name.as_ptr().cast());
                    let fragment = gl::GetUniformLocation(pipeline, frag_name.as_ptr().cast());

                    VariableLocation::Ubo(UniformLocation {
                        vertex,
                        fragment
                    })
                }
            }
            MemberOffset::PushConstant(_) => {
                let vert_name = format!("LIBRA_PUSH_VERTEX_INSTANCE.{}\0", meta.id());
                let frag_name = format!("LIBRA_PUSH_FRAGMENT_INSTANCE.{}\0", meta.id());
                unsafe {
                    let vertex = gl::GetUniformLocation(pipeline, vert_name.as_ptr().cast());
                    let fragment = gl::GetUniformLocation(pipeline, frag_name.as_ptr().cast());

                    VariableLocation::Push(UniformLocation {
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
    common: FilterCommon,
    quad_vao: GLuint,
}

pub struct FilterCommon {
    semantics: ReflectSemantics,
    pub(crate) preset: ShaderPreset,
    original_history: Vec<Framebuffer>,
    history: Vec<Texture>,
    feedback: Vec<Texture>,
    pub(crate) luts: FxHashMap<usize, Texture>,
    outputs: Vec<Framebuffer>,
    pub(crate) quad_vbo: GLuint,
}

impl FilterChain {
    pub fn load(path: impl AsRef<Path>) -> Result<FilterChain, Box<dyn Error>> {
        let preset = ShaderPreset::try_parse(path)?;
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
                index
            });

            uniform_semantics.insert(format!("{}Size", texture.name), UniformSemantic::Texture(SemanticMap {
                semantics: TextureSemantics::User,
                index
            }));
        }

        let semantics = ReflectSemantics {
            uniform_semantics,
            non_uniform_semantics: texture_semantics
        };

        let mut filters = Vec::new();
        let mut output_framebuffers = Vec::new();

        // initialize passes
        for (index, (config, source, mut reflect)) in passes.into_iter().enumerate() {
            let mut semantics = semantics.clone();

            let reflection = reflect.reflect(index, &semantics)?;
            let glsl = reflect.compile(GlVersion::V4_60)?;

            let vertex_resources = glsl.context.compiler.vertex.get_shader_resources()?;

            // todo: split this out.
            let (program, ubo_location) = unsafe {
                let vertex = util::gl_compile_shader(gl::VERTEX_SHADER, glsl.vertex.as_str());
                let fragment = util::gl_compile_shader(gl::FRAGMENT_SHADER, glsl.fragment.as_str());

                let program = gl::CreateProgram();
                gl::AttachShader(program, vertex);
                gl::AttachShader(program, fragment);

                for res in &vertex_resources.stage_inputs {
                    let loc = glsl.context.compiler.vertex.get_decoration(res.id, Decoration::Location)?;
                    let loc_name = format!("LIBRA_ATTRIBUTE_{loc}\0");
                    eprintln!("{loc_name}");
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

                gl::UseProgram(program);

                for binding in &glsl.context.sampler_bindings {
                    let loc_name = format!("LIBRA_TEXTURE_{}\0", *binding);
                    let location = gl::GetUniformLocation(program, loc_name.as_str().as_ptr().cast());
                    if location >= 0 {
                        // eprintln!("setting sampler {location} to sample from {binding}");
                        gl::Uniform1i(location, *binding as GLint);
                    }
                }

                gl::UseProgram(0);
                (program, UniformLocation {
                    vertex: gl::GetUniformBlockIndex(program, b"LIBRA_UBO_VERTEX\0".as_ptr().cast()),
                    fragment: gl::GetUniformBlockIndex(program, b"LIBRA_UBO_FRAGMENT\0".as_ptr().cast()),
                })
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
                locations.insert(UniformBinding::Parameter(param.id.clone()),
                                 (FilterChain::reflect_uniform_location(program, param), param.offset));
            }

            for (semantics, param) in &reflection.meta.variable_meta {
                locations.insert(UniformBinding::SemanticVariable(semantics.clone()),
                                 (FilterChain::reflect_uniform_location(program, param), param.offset));
            }

            for (semantics, param) in &reflection.meta.texture_size_meta {
                locations.insert(UniformBinding::TextureSize(semantics.clone()),
                                 (FilterChain::reflect_uniform_location(program, param), param.offset));
            }

            // need output framebuffers.
            output_framebuffers.push(Framebuffer::new(1));

            // eprintln!("{:#?}", semantics);
            // eprintln!("{:#?}", reflection.meta);
            // eprintln!("{:#?}", locations);
            // eprintln!("{:#?}", reflection.push_constant);
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
                variable_bindings: locations,
                source,
                config: config.clone()
            });
        }

        // load luts
        let mut luts = FxHashMap::default();

        for (index, texture) in preset.textures.iter().enumerate() {
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

            luts.insert(index, Texture {
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

        let mut quad_vbo = 0;
        unsafe {
            gl::GenBuffers(1, &mut quad_vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, quad_vbo);
            gl::BufferData(gl::ARRAY_BUFFER, std::mem::size_of_val(QUAD_VBO_DATA) as GLsizeiptr,
                           QUAD_VBO_DATA.as_ptr().cast(), gl::STATIC_DRAW);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        let mut quad_vao = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut quad_vao);
        }

        Ok(FilterChain {
            passes: filters,
            quad_vao,
            common: FilterCommon {
                semantics,
                preset,
                original_history: vec![],
                history: vec![],
                feedback: vec![],
                luts,
                outputs: output_framebuffers,
                quad_vbo,
            }
        })
    }

    pub fn frame(&mut self, count: u32, vp: &Viewport, input: GlImage, clear: bool) {
        if self.passes.is_empty() {
            return;
        }

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindVertexArray(self.quad_vao);
        }

        // todo: copy framebuffer
        // shader_gl3: 2067
        let filter = self.common.preset.shaders.first().map(|f| f.filter).unwrap_or_default();
        let wrap_mode = self.common.preset.shaders.first().map(|f| f.wrap_mode).unwrap_or_default();

        let original = Texture {
            image: input,
            filter,
            mip_filter: filter,
            wrap_mode
        };

        let mut source = original.clone();

        let passes_len = self.passes.len();
        let (pass, last) = self.passes.split_at_mut(passes_len - 1);

        for (index, pass) in pass.iter_mut().enumerate() {
            {
                let target = &mut self.common.outputs[index];
                let framebuffer_size = target.scale(pass.config.scaling.clone(), pass.get_format(), vp, &original, &source);
            }
            let target = &self.common.outputs[index];
            pass.draw(&self.common, count, 1, vp, &original, &source, RenderTarget::new(target, None));
            let target = target.as_texture(pass.config.filter, pass.config.wrap_mode);

            // todo: update-pass-outputs
            source = target;
            // passes.build_semantics(&self, None, count, 1, vp, &original, &source);
        }

        assert_eq!(last.len(), 1);
        for pass in last {
            source.filter = pass.config.filter;
            source.mip_filter = pass.config.filter;
            pass.draw(&self.common, count, 1, vp, &original, &source, RenderTarget::new(&vp.output, vp.mvp));
        }

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindVertexArray(0);
        }
        // todo: deal with the mess that is frame history
    }
}
