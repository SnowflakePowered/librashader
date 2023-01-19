use crate::binding::{GlUniformStorage, UniformLocation, VariableLocation};
use crate::error::FilterChainError;
use crate::filter_pass::{FilterPass, UniformOffset};
use crate::gl::{DrawQuad, Framebuffer, FramebufferInterface, GLInterface, LoadLut, UboRing};
use crate::options::{FilterChainOptionsGL, FrameOptionsGL};
use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;
use crate::texture::InputTexture;
use crate::util::{gl_get_version, gl_u16_to_version};
use crate::{error, util, GLImage};
use gl::types::{GLint, GLuint};
use librashader_common::{FilterMode, Viewport, WrapMode};

use librashader_presets::ShaderPreset;
use librashader_reflect::back::cross::GlslVersion;
use librashader_reflect::back::targets::GLSL;
use librashader_reflect::back::CompileShader;
use librashader_reflect::front::GlslangCompilation;
use librashader_reflect::reflect::semantics::{
    MemberOffset, ShaderSemantics, TextureSemantics, UniformBinding, UniformMeta,
};

use librashader_reflect::reflect::presets::CompilePreset;
use librashader_reflect::reflect::ReflectShader;
use librashader_runtime::decl_shader_pass_meta;
use rustc_hash::FxHashMap;
use spirv_cross::spirv::Decoration;
use std::collections::VecDeque;

pub(crate) struct FilterChainImpl<T: GLInterface> {
    pub(crate) common: FilterCommon,
    passes: Box<[FilterPass<T>]>,
    draw_quad: T::DrawQuad,
    output_framebuffers: Box<[Framebuffer]>,
    feedback_framebuffers: Box<[Framebuffer]>,
    history_framebuffers: VecDeque<Framebuffer>,
}

pub(crate) struct FilterCommon {
    // semantics: ReflectSemantics,
    pub config: FilterMutable,
    pub luts: FxHashMap<usize, InputTexture>,
    pub samplers: SamplerSet,
    pub output_textures: Box<[InputTexture]>,
    pub feedback_textures: Box<[InputTexture]>,
    pub history_textures: Box<[InputTexture]>,
    pub disable_mipmaps: bool,
}

pub struct FilterMutable {
    pub(crate) passes_enabled: usize,
    pub(crate) parameters: FxHashMap<String, f32>,
}

impl<T: GLInterface> FilterChainImpl<T> {
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

                    VariableLocation::Ubo(UniformLocation { vertex, fragment })
                }
            }
            MemberOffset::PushConstant(_) => {
                let vert_name = format!("LIBRA_PUSH_VERTEX_INSTANCE.{}\0", meta.id());
                let frag_name = format!("LIBRA_PUSH_FRAGMENT_INSTANCE.{}\0", meta.id());
                unsafe {
                    let vertex = gl::GetUniformLocation(pipeline, vert_name.as_ptr().cast());
                    let fragment = gl::GetUniformLocation(pipeline, frag_name.as_ptr().cast());

                    VariableLocation::Push(UniformLocation { vertex, fragment })
                }
            }
        }
    }
}

decl_shader_pass_meta!(type ShaderPassMeta = <GLSL, GlslangCompilation>);

impl<T: GLInterface> FilterChainImpl<T> {
    /// Load a filter chain from a pre-parsed `ShaderPreset`.
    pub(crate) fn load_from_preset(
        preset: ShaderPreset,
        options: Option<&FilterChainOptionsGL>,
    ) -> error::Result<Self> {
        let (passes, semantics) = GLSL::compile_preset_passes::<
            GlslangCompilation,
            FilterChainError,
        >(preset.shaders, &preset.textures)?;

        let version = options
            .map(|o| gl_u16_to_version(o.gl_version))
            .unwrap_or_else(gl_get_version);

        // initialize passes
        let filters = Self::init_passes(version, passes, &semantics)?;

        let default_filter = filters.first().map(|f| f.config.filter).unwrap_or_default();
        let default_wrap = filters
            .first()
            .map(|f| f.config.wrap_mode)
            .unwrap_or_default();

        let samplers = SamplerSet::new();

        // initialize output framebuffers
        let mut output_framebuffers = Vec::new();
        output_framebuffers.resize_with(filters.len(), || T::FramebufferInterface::new(1));
        let mut output_textures = Vec::new();
        output_textures.resize_with(filters.len(), InputTexture::default);

        // initialize feedback framebuffers
        let mut feedback_framebuffers = Vec::new();
        feedback_framebuffers.resize_with(filters.len(), || T::FramebufferInterface::new(1));
        let mut feedback_textures = Vec::new();
        feedback_textures.resize_with(filters.len(), InputTexture::default);

        // load luts
        let luts = T::LoadLut::load_luts(&preset.textures)?;

        let (history_framebuffers, history_textures) =
            FilterChainImpl::init_history(&filters, default_filter, default_wrap);

        // create vertex objects
        let draw_quad = T::DrawQuad::new();

        Ok(FilterChainImpl {
            passes: filters,
            output_framebuffers: output_framebuffers.into_boxed_slice(),
            feedback_framebuffers: feedback_framebuffers.into_boxed_slice(),
            history_framebuffers,
            draw_quad,
            common: FilterCommon {
                config: FilterMutable {
                    passes_enabled: preset.shader_count as usize,
                    parameters: preset
                        .parameters
                        .into_iter()
                        .map(|param| (param.name, param.value))
                        .collect(),
                },
                disable_mipmaps: options.map_or(false, |o| o.force_no_mipmaps),
                luts,
                samplers,
                output_textures: output_textures.into_boxed_slice(),
                feedback_textures: feedback_textures.into_boxed_slice(),
                history_textures,
            },
        })
    }

    fn init_passes(
        version: GlslVersion,
        passes: Vec<ShaderPassMeta>,
        semantics: &ShaderSemantics,
    ) -> error::Result<Box<[FilterPass<T>]>> {
        let mut filters = Vec::new();

        // initialize passes
        for (index, (config, source, mut reflect)) in passes.into_iter().enumerate() {
            let reflection = reflect.reflect(index, semantics)?;
            let glsl = reflect.compile(version)?;

            let vertex_resources = glsl.context.artifact.vertex.get_shader_resources()?;

            // todo: split this out.
            let (program, ubo_location) = unsafe {
                let vertex = util::gl_compile_shader(gl::VERTEX_SHADER, glsl.vertex.as_str())?;
                let fragment =
                    util::gl_compile_shader(gl::FRAGMENT_SHADER, glsl.fragment.as_str())?;

                let program = gl::CreateProgram();
                gl::AttachShader(program, vertex);
                gl::AttachShader(program, fragment);

                for res in vertex_resources.stage_inputs {
                    let loc = glsl
                        .context
                        .artifact
                        .vertex
                        .get_decoration(res.id, Decoration::Location)?;
                    let mut name = res.name;
                    name.push('\0');

                    gl::BindAttribLocation(program, loc, name.as_str().as_ptr().cast())
                }
                gl::LinkProgram(program);
                gl::DeleteShader(vertex);
                gl::DeleteShader(fragment);

                let mut status = 0;
                gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
                if status != 1 {
                    return Err(FilterChainError::GLLinkError);
                }

                gl::UseProgram(program);

                for (name, binding) in &glsl.context.sampler_bindings {
                    let location = gl::GetUniformLocation(program, name.as_str().as_ptr().cast());
                    if location >= 0 {
                        // eprintln!("setting sampler {location} to sample from {binding}");
                        gl::Uniform1i(location, *binding as GLint);
                    }
                }

                gl::UseProgram(0);
                (
                    program,
                    UniformLocation {
                        vertex: gl::GetUniformBlockIndex(
                            program,
                            b"LIBRA_UBO_VERTEX\0".as_ptr().cast(),
                        ),
                        fragment: gl::GetUniformBlockIndex(
                            program,
                            b"LIBRA_UBO_FRAGMENT\0".as_ptr().cast(),
                        ),
                    },
                )
            };

            let ubo_ring = if let Some(ubo) = &reflection.ubo {
                let ring = UboRing::new(ubo.size);
                Some(ring)
            } else {
                None
            };

            let uniform_storage = GlUniformStorage::new(
                reflection
                    .ubo
                    .as_ref()
                    .map(|ubo| ubo.size as usize)
                    .unwrap_or(0),
                reflection
                    .push_constant
                    .as_ref()
                    .map(|push| push.size as usize)
                    .unwrap_or(0),
            );

            let mut uniform_bindings = FxHashMap::default();
            for param in reflection.meta.parameter_meta.values() {
                uniform_bindings.insert(
                    UniformBinding::Parameter(param.id.clone()),
                    UniformOffset::new(
                        Self::reflect_uniform_location(program, param),
                        param.offset,
                    ),
                );
            }

            for (semantics, param) in &reflection.meta.unique_meta {
                uniform_bindings.insert(
                    UniformBinding::SemanticVariable(*semantics),
                    UniformOffset::new(
                        Self::reflect_uniform_location(program, param),
                        param.offset,
                    ),
                );
            }

            for (semantics, param) in &reflection.meta.texture_size_meta {
                uniform_bindings.insert(
                    UniformBinding::TextureSize(*semantics),
                    UniformOffset::new(
                        Self::reflect_uniform_location(program, param),
                        param.offset,
                    ),
                );
            }

            // eprintln!("{:#?}", reflection.meta.texture_meta);
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
                uniform_storage,
                uniform_bindings,
                source,
                config,
            });
        }

        Ok(filters.into_boxed_slice())
    }

    fn init_history(
        filters: &[FilterPass<T>],
        filter: FilterMode,
        wrap_mode: WrapMode,
    ) -> (VecDeque<Framebuffer>, Box<[InputTexture]>) {
        let mut required_images = 0;

        for pass in filters {
            // If a shader uses history size, but not history, we still need to keep the texture.
            let texture_count = pass
                .reflection
                .meta
                .texture_meta
                .iter()
                .filter(|(semantics, _)| semantics.semantics == TextureSemantics::OriginalHistory)
                .count();
            let texture_size_count = pass
                .reflection
                .meta
                .texture_size_meta
                .iter()
                .filter(|(semantics, _)| semantics.semantics == TextureSemantics::OriginalHistory)
                .count();

            required_images = std::cmp::max(required_images, texture_count);
            required_images = std::cmp::max(required_images, texture_size_count);
        }

        // not using frame history;
        if required_images <= 1 {
            // println!("[history] not using frame history");
            return (VecDeque::new(), Box::new([]));
        }

        // history0 is aliased with the original

        // eprintln!("[history] using frame history with {required_images} images");
        let mut framebuffers = VecDeque::with_capacity(required_images);
        framebuffers.resize_with(required_images, || T::FramebufferInterface::new(1));

        let mut history_textures = Vec::new();
        history_textures.resize_with(required_images, || InputTexture {
            image: Default::default(),
            filter,
            mip_filter: filter,
            wrap_mode,
        });

        (framebuffers, history_textures.into_boxed_slice())
    }

    fn push_history(&mut self, input: &GLImage) -> error::Result<()> {
        if let Some(mut back) = self.history_framebuffers.pop_back() {
            if back.size != input.size || (input.format != 0 && input.format != back.format) {
                // eprintln!("[history] resizing");
                T::FramebufferInterface::init(&mut back, input.size, input.format)?;
            }

            back.copy_from::<T::FramebufferInterface>(input)?;

            self.history_framebuffers.push_front(back)
        }

        Ok(())
    }

    /// Process a frame with the input image.
    ///
    /// When this frame returns, GL_FRAMEBUFFER is bound to 0.
    pub fn frame(
        &mut self,
        count: usize,
        viewport: &Viewport<&Framebuffer>,
        input: &GLImage,
        options: Option<&FrameOptionsGL>,
    ) -> error::Result<()> {
        // limit number of passes to those enabled.
        let max = std::cmp::min(self.passes.len(), self.common.config.passes_enabled);
        let passes = &mut self.passes[0..max];

        if let Some(options) = options {
            if options.clear_history {
                for framebuffer in &self.history_framebuffers {
                    framebuffer.clear::<T::FramebufferInterface, true>()
                }
            }
        }

        if passes.is_empty() {
            return Ok(());
        }
        let frame_direction = options.map(|f| f.frame_direction).unwrap_or(1);

        // do not need to rebind FBO 0 here since first `draw` will
        // bind automatically.
        self.draw_quad.bind_vertices();

        let filter = passes[0].config.filter;
        let wrap_mode = passes[0].config.wrap_mode;

        // update history
        for (texture, fbo) in self
            .common
            .history_textures
            .iter_mut()
            .zip(self.history_framebuffers.iter())
        {
            texture.image = fbo.as_texture(filter, wrap_mode).image;
        }

        for ((texture, fbo), pass) in self
            .common
            .feedback_textures
            .iter_mut()
            .zip(self.feedback_framebuffers.iter())
            .zip(passes.iter())
        {
            texture.image = fbo
                .as_texture(pass.config.filter, pass.config.wrap_mode)
                .image;
        }

        // shader_gl3: 2067
        let original = InputTexture {
            image: *input,
            filter,
            mip_filter: filter,
            wrap_mode,
        };

        let mut source = original;

        // rescale render buffers to ensure all bindings are valid.
        let mut iterator = passes.iter_mut().enumerate().peekable();
        while let Some((index, pass)) = iterator.next() {
            let should_mipmap = iterator
                .peek()
                .map(|(_, p)| p.config.mipmap_input)
                .unwrap_or(false);

            self.output_framebuffers[index].scale::<T::FramebufferInterface>(
                pass.config.scaling.clone(),
                pass.get_format(),
                viewport,
                &original,
                &source,
                should_mipmap,
            )?;

            self.feedback_framebuffers[index].scale::<T::FramebufferInterface>(
                pass.config.scaling.clone(),
                pass.get_format(),
                viewport,
                &original,
                &source,
                should_mipmap,
            )?;
        }

        let passes_len = passes.len();
        let (pass, last) = passes.split_at_mut(passes_len - 1);

        for (index, pass) in pass.iter_mut().enumerate() {
            let target = &self.output_framebuffers[index];
            pass.draw(
                index,
                &self.common,
                if pass.config.frame_count_mod > 0 {
                    count % pass.config.frame_count_mod as usize
                } else {
                    count
                } as u32,
                frame_direction,
                viewport,
                &original,
                &source,
                RenderTarget::new(target, None, 0, 0),
            );

            let target = target.as_texture(pass.config.filter, pass.config.wrap_mode);
            self.common.output_textures[index] = target;
            source = target;
        }

        // try to hint the optimizer
        assert_eq!(last.len(), 1);
        if let Some(pass) = last.iter_mut().next() {
            source.filter = pass.config.filter;
            source.mip_filter = pass.config.filter;

            pass.draw(
                passes_len - 1,
                &self.common,
                if pass.config.frame_count_mod > 0 {
                    count % pass.config.frame_count_mod as usize
                } else {
                    count
                } as u32,
                frame_direction,
                viewport,
                &original,
                &source,
                viewport.into(),
            );
            self.common.output_textures[passes_len - 1] = viewport
                .output
                .as_texture(pass.config.filter, pass.config.wrap_mode);
        }

        // swap feedback framebuffers with output
        std::mem::swap(
            &mut self.output_framebuffers,
            &mut self.feedback_framebuffers,
        );

        self.push_history(input)?;

        self.draw_quad.unbind_vertices();

        Ok(())
    }
}
