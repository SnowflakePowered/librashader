use crate::texture::{D3D11InputView, InputTexture, LutTexture};
use librashader_common::{ImageFormat, Size, Viewport};

use librashader_presets::{ShaderPreset, TextureConfig};
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::back::{CompileReflectShader, CompileShader};
use librashader_reflect::front::GlslangCompilation;
use librashader_reflect::reflect::semantics::{ShaderSemantics, TextureSemantics, UniformBinding};
use librashader_reflect::reflect::ReflectShader;
use librashader_runtime::image::{Image, UVDirection};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;

use std::path::Path;

use crate::error::{assume_d3d11_init, FilterChainError};
use crate::filter_pass::{ConstantBufferBinding, FilterPass};
use crate::framebuffer::OwnedFramebuffer;
use crate::options::{FilterChainOptionsD3D11, FrameOptionsD3D11};
use crate::quad_render::DrawQuad;
use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;
use crate::util::d3d11_compile_bound_shader;
use crate::{error, util, D3D11OutputView};
use librashader_reflect::reflect::presets::{CompilePresetTarget, ShaderPassArtifact};
use librashader_runtime::binding::TextureInput;
use librashader_runtime::uniforms::UniformStorage;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, D3D11_BIND_CONSTANT_BUFFER, D3D11_BUFFER_DESC,
    D3D11_CPU_ACCESS_WRITE, D3D11_RESOURCE_MISC_FLAG, D3D11_RESOURCE_MISC_GENERATE_MIPS,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R8G8B8A8_UNORM;
use librashader_runtime::quad::{IDENTITY_MVP, QuadType};

pub struct FilterMutable {
    pub(crate) passes_enabled: usize,
    pub(crate) parameters: FxHashMap<String, f32>,
}

type ShaderPassMeta = ShaderPassArtifact<impl CompileReflectShader<HLSL, GlslangCompilation>>;

/// A Direct3D 11 filter chain.
pub struct FilterChainD3D11 {
    pub(crate) common: FilterCommon,
    pub(crate) passes: Vec<FilterPass>,
    pub(crate) output_framebuffers: Box<[OwnedFramebuffer]>,
    pub(crate) feedback_framebuffers: Box<[OwnedFramebuffer]>,
    pub(crate) history_framebuffers: VecDeque<OwnedFramebuffer>,
}

pub(crate) struct Direct3D11 {
    pub(crate) device: ID3D11Device,
    pub(crate) current_context: ID3D11DeviceContext,
    pub(crate) immediate_context: ID3D11DeviceContext,
    pub context_is_deferred: bool,
}

pub(crate) struct FilterCommon {
    pub(crate) d3d11: Direct3D11,
    pub(crate) luts: FxHashMap<usize, LutTexture>,
    pub samplers: SamplerSet,
    pub output_textures: Box<[Option<InputTexture>]>,
    pub feedback_textures: Box<[Option<InputTexture>]>,
    pub history_textures: Box<[Option<InputTexture>]>,
    pub config: FilterMutable,
    pub disable_mipmaps: bool,
    pub(crate) draw_quad: DrawQuad,
}

impl FilterChainD3D11 {
    /// Load the shader preset at the given path into a filter chain.
    pub fn load_from_path(
        device: &ID3D11Device,
        path: impl AsRef<Path>,
        options: Option<&FilterChainOptionsD3D11>,
    ) -> error::Result<FilterChainD3D11> {
        // load passes from preset
        let preset = ShaderPreset::try_parse(path)?;
        Self::load_from_preset(device, preset, options)
    }

    /// Load a filter chain from a pre-parsed `ShaderPreset`.
    pub fn load_from_preset(
        device: &ID3D11Device,
        preset: ShaderPreset,
        options: Option<&FilterChainOptionsD3D11>,
    ) -> error::Result<FilterChainD3D11> {
        let (passes, semantics) = HLSL::compile_preset_passes::<
            GlslangCompilation,
            FilterChainError,
        >(preset.shaders, &preset.textures)?;

        let use_deferred_context = options.map(|f| f.use_deferred_context).unwrap_or(false);

        let samplers = SamplerSet::new(device)?;

        // initialize passes
        let filters = FilterChainD3D11::init_passes(device, passes, &semantics)?;

        let immediate_context = unsafe { device.GetImmediateContext()? };

        let current_context = if use_deferred_context {
            // check if device supports deferred contexts
            if let Err(_) = unsafe { device.CreateDeferredContext(0, None) } {
                immediate_context.clone()
            } else {
                let mut context = None;
                unsafe { device.CreateDeferredContext(0, Some(&mut context))? };
                assume_d3d11_init!(context, "CreateDeferredContext");
                context
            }
        } else {
            immediate_context.clone()
        };

        // initialize output framebuffers
        let mut output_framebuffers = Vec::new();
        output_framebuffers.resize_with(filters.len(), || {
            OwnedFramebuffer::new(
                device,
                &current_context,
                Size::new(1, 1),
                ImageFormat::R8G8B8A8Unorm,
                false,
            )
        });

        // resolve all results
        let output_framebuffers = output_framebuffers
            .into_iter()
            .collect::<error::Result<Vec<OwnedFramebuffer>>>()?;

        let mut output_textures = Vec::new();
        output_textures.resize_with(filters.len(), || None);
        //
        // // initialize feedback framebuffers
        let mut feedback_framebuffers = Vec::new();
        feedback_framebuffers.resize_with(filters.len(), || {
            OwnedFramebuffer::new(
                device,
                &current_context,
                Size::new(1, 1),
                ImageFormat::R8G8B8A8Unorm,
                false,
            )
        });
        // resolve all results
        let feedback_framebuffers = feedback_framebuffers
            .into_iter()
            .collect::<error::Result<Vec<OwnedFramebuffer>>>()?;

        let mut feedback_textures = Vec::new();
        feedback_textures.resize_with(filters.len(), || None);

        // load luts
        let luts = FilterChainD3D11::load_luts(device, &current_context, &preset.textures)?;

        let (history_framebuffers, history_textures) =
            FilterChainD3D11::init_history(device, &current_context, &filters)?;

        let draw_quad = DrawQuad::new(device, &current_context)?;

        // todo: make vbo: d3d11.c 1376
        Ok(FilterChainD3D11 {
            passes: filters,
            output_framebuffers: output_framebuffers.into_boxed_slice(),
            feedback_framebuffers: feedback_framebuffers.into_boxed_slice(),
            history_framebuffers,
            common: FilterCommon {
                d3d11: Direct3D11 {
                    device: device.clone(),
                    current_context,
                    immediate_context,
                    context_is_deferred: use_deferred_context,
                },
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
                draw_quad,
            },
        })
    }
}

impl FilterChainD3D11 {
    fn create_constant_buffer(device: &ID3D11Device, size: u32) -> error::Result<ID3D11Buffer> {
        unsafe {
            let mut buffer = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: size,
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                    MiscFlags: D3D11_RESOURCE_MISC_FLAG(0),
                    StructureByteStride: 0,
                },
                None,
                Some(&mut buffer),
            )?;
            assume_d3d11_init!(buffer, "CreateBuffer");
            Ok(buffer)
        }
    }

    fn init_passes(
        device: &ID3D11Device,
        passes: Vec<ShaderPassMeta>,
        semantics: &ShaderSemantics,
    ) -> error::Result<Vec<FilterPass>> {
        let mut filters = Vec::new();

        for (index, (config, source, mut reflect)) in passes.into_iter().enumerate() {
            let reflection = reflect.reflect(index, semantics)?;
            let hlsl = reflect.compile(None)?;

            let vertex_dxbc =
                util::d3d_compile_shader(hlsl.vertex.as_bytes(), b"main\0", b"vs_5_0\0")?;
            let vs = d3d11_compile_bound_shader(
                device,
                &vertex_dxbc,
                None,
                ID3D11Device::CreateVertexShader,
            )?;

            let ia_desc = DrawQuad::get_spirv_cross_vbo_desc();
            let vao = util::d3d11_create_input_layout(device, &ia_desc, &vertex_dxbc)?;

            let fragment_dxbc =
                util::d3d_compile_shader(hlsl.fragment.as_bytes(), b"main\0", b"ps_5_0\0")?;
            let ps = d3d11_compile_bound_shader(
                device,
                &fragment_dxbc,
                None,
                ID3D11Device::CreatePixelShader,
            )?;

            let ubo_cbuffer = if let Some(ubo) = &reflection.ubo && ubo.size != 0 {
                let buffer = FilterChainD3D11::create_constant_buffer(device, ubo.size)?;
                Some(ConstantBufferBinding {
                    binding: ubo.binding,
                    size: ubo.size,
                    stage_mask: ubo.stage_mask,
                    buffer,
                })
            } else {
                None
            };

            let push_cbuffer = if let Some(push) = &reflection.push_constant && push.size != 0 {
                let buffer = FilterChainD3D11::create_constant_buffer(device, push.size)?;
                Some(ConstantBufferBinding {
                    binding: if ubo_cbuffer.is_some() { 1 } else { 0 },
                    size: push.size,
                    stage_mask: push.stage_mask,
                    buffer,
                })
            } else {
                None
            };

            let uniform_storage = UniformStorage::new(
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
                uniform_bindings.insert(UniformBinding::Parameter(param.id.clone()), param.offset);
            }

            for (semantics, param) in &reflection.meta.unique_meta {
                uniform_bindings.insert(UniformBinding::SemanticVariable(*semantics), param.offset);
            }

            for (semantics, param) in &reflection.meta.texture_size_meta {
                uniform_bindings.insert(UniformBinding::TextureSize(*semantics), param.offset);
            }

            filters.push(FilterPass {
                reflection,
                compiled: hlsl,
                vertex_shader: vs,
                vertex_layout: vao,
                pixel_shader: ps,
                uniform_bindings,
                uniform_storage,
                uniform_buffer: ubo_cbuffer,
                push_buffer: push_cbuffer,
                source,
                config: config.clone(),
            })
        }
        Ok(filters)
    }

    fn init_history(
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        filters: &Vec<FilterPass>,
    ) -> error::Result<(VecDeque<OwnedFramebuffer>, Box<[Option<InputTexture>]>)> {
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
            return Ok((VecDeque::new(), Box::new([])));
        }

        // history0 is aliased with the original

        // eprintln!("[history] using frame history with {required_images} images");
        let mut framebuffers = VecDeque::with_capacity(required_images);
        framebuffers.resize_with(required_images, || {
            OwnedFramebuffer::new(
                device,
                context,
                Size::new(1, 1),
                ImageFormat::R8G8B8A8Unorm,
                false,
            )
        });

        let framebuffers = framebuffers
            .into_iter()
            .collect::<error::Result<VecDeque<OwnedFramebuffer>>>()?;

        let mut history_textures = Vec::new();
        history_textures.resize_with(required_images, || None);

        Ok((framebuffers, history_textures.into_boxed_slice()))
    }

    fn push_history(&mut self, input: &D3D11InputView) -> error::Result<()> {
        if let Some(mut back) = self.history_framebuffers.pop_back() {
            back.copy_from(input)?;
            self.history_framebuffers.push_front(back)
        }

        Ok(())
    }

    fn load_luts(
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        textures: &[TextureConfig],
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
        let mut luts = FxHashMap::default();

        for (index, texture) in textures.iter().enumerate() {
            let image = Image::load(&texture.path, UVDirection::TopLeft)?;
            let desc = D3D11_TEXTURE2D_DESC {
                Width: image.size.width,
                Height: image.size.height,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                Usage: D3D11_USAGE_DEFAULT,
                MiscFlags: if texture.mipmap {
                    D3D11_RESOURCE_MISC_GENERATE_MIPS
                } else {
                    D3D11_RESOURCE_MISC_FLAG(0)
                },
                ..Default::default()
            };

            let texture = LutTexture::new(
                device,
                context,
                &image,
                desc,
                texture.filter_mode,
                texture.wrap_mode,
            )?;
            luts.insert(index, texture);
        }
        Ok(luts)
    }

    /// Process a frame with the input image.
    pub fn frame(
        &mut self,
        input: D3D11InputView,
        viewport: &Viewport<D3D11OutputView>,
        frame_count: usize,
        options: Option<&FrameOptionsD3D11>,
    ) -> error::Result<()> {
        let max = std::cmp::min(self.passes.len(), self.common.config.passes_enabled);
        let passes = &mut self.passes[0..max];
        if let Some(options) = options {
            if options.clear_history {
                for framebuffer in &mut self.history_framebuffers {
                    framebuffer.init(Size::new(1, 1), ImageFormat::R8G8B8A8Unorm)?;
                }
            }
        }

        if passes.is_empty() {
            return Ok(());
        }

        let frame_direction = options.map(|f| f.frame_direction).unwrap_or(1);
        let filter = passes[0].config.filter;
        let wrap_mode = passes[0].config.wrap_mode;

        for ((texture, fbo), pass) in self
            .common
            .feedback_textures
            .iter_mut()
            .zip(self.feedback_framebuffers.iter())
            .zip(passes.iter())
        {
            *texture = Some(InputTexture::from_framebuffer(
                fbo,
                pass.config.wrap_mode,
                pass.config.filter,
            )?);
        }

        for (texture, fbo) in self
            .common
            .history_textures
            .iter_mut()
            .zip(self.history_framebuffers.iter())
        {
            *texture = Some(InputTexture::from_framebuffer(fbo, wrap_mode, filter)?);
        }

        let original = InputTexture {
            view: input.clone(),
            filter,
            wrap_mode,
        };

        let mut source = original.clone();
        let mut iterator = passes.iter_mut().enumerate().peekable();

        // rescale render buffers to ensure all bindings are valid.
        let mut source_size = source.size();
        while let Some((index, pass)) = iterator.next() {
            let should_mipmap = iterator
                .peek()
                .map(|(_, p)| p.config.mipmap_input)
                .unwrap_or(false);

            let next_size = self.output_framebuffers[index].scale(
                pass.config.scaling.clone(),
                pass.get_format(),
                &viewport.output.size,
                &source_size,
                should_mipmap,
            )?;

            self.feedback_framebuffers[index].scale(
                pass.config.scaling.clone(),
                pass.get_format(),
                &viewport.output.size,
                &source_size,
                should_mipmap,
            )?;

            source_size = next_size;
        }

        let passes_len = passes.len();
        let (pass, last) = passes.split_at_mut(passes_len - 1);

        for (index, pass) in pass.iter_mut().enumerate() {
            source.filter = pass.config.filter;
            source.wrap_mode = pass.config.wrap_mode;
            let target = &self.output_framebuffers[index];
            let size = target.size;
            pass.draw(
                index,
                &self.common,
                if pass.config.frame_count_mod > 0 {
                    frame_count % pass.config.frame_count_mod as usize
                } else {
                    frame_count
                } as u32,
                frame_direction,
                viewport,
                &original,
                &source,
                RenderTarget::new(target.as_output_framebuffer()?, Some(IDENTITY_MVP)),
                QuadType::Offscreen
            )?;

            source = InputTexture {
                view: D3D11InputView {
                    handle: target.create_shader_resource_view()?,
                    size,
                },
                filter: pass.config.filter,
                wrap_mode: pass.config.wrap_mode,
            };
            self.common.output_textures[index] = Some(source.clone());
        }

        // try to hint the optimizer
        assert_eq!(last.len(), 1);
        if let Some(pass) = last.iter_mut().next() {
            source.filter = pass.config.filter;
            source.wrap_mode = pass.config.wrap_mode;
            pass.draw(
                passes_len - 1,
                &self.common,
                if pass.config.frame_count_mod > 0 {
                    frame_count % pass.config.frame_count_mod as usize
                } else {
                    frame_count
                } as u32,
                frame_direction,
                viewport,
                &original,
                &source,
                viewport.into(),
                QuadType::Final
            )?;
        }

        std::mem::swap(
            &mut self.output_framebuffers,
            &mut self.feedback_framebuffers,
        );

        self.push_history(&input)?;

        if self.common.d3d11.context_is_deferred {
            unsafe {
                let mut command_list = None;
                self.common
                    .d3d11
                    .current_context
                    .FinishCommandList(false, Some(&mut command_list))?;
                assume_d3d11_init!(command_list, "FinishCommandList");
                self.common
                    .d3d11
                    .immediate_context
                    .ExecuteCommandList(&command_list, true);
            }
        }

        Ok(())
    }
}
