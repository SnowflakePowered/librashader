use std::collections::VecDeque;
use crate::texture::{DxImageView, OwnedTexture, Texture};
use librashader_common::image::Image;
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_preprocess::ShaderSource;
use librashader_presets::{ShaderPassConfig, ShaderPreset, TextureConfig};
use librashader_reflect::back::cross::GlslangHlslContext;
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::back::{CompilerBackend, CompileShader, FromCompilation};
use librashader_reflect::front::shaderc::GlslangCompilation;
use librashader_reflect::reflect::semantics::{ReflectSemantics, SemanticMap, TextureSemantics, UniformBinding, UniformSemantic, VariableSemantics};
use librashader_reflect::reflect::ReflectShader;
use rustc_hash::FxHashMap;
use std::error::Error;
use std::path::Path;
use bytemuck::offset_of;
use windows::core::PCSTR;
use windows::s;
use windows::Win32::Graphics::Direct3D11::{D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_SHADER_RESOURCE, D3D11_BUFFER_DESC, D3D11_CPU_ACCESS_WRITE, D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_RESOURCE_MISC_FLAG, D3D11_RESOURCE_MISC_GENERATE_MIPS, D3D11_SAMPLER_DESC, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC, ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11ShaderResourceView};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};
use librashader_runtime::uniforms::UniformStorage;
use crate::filter_pass::{ConstantBufferBinding, FilterPass};
use crate::framebuffer::{OutputFramebuffer, OwnedFramebuffer};
use crate::quad_render::DrawQuad;
use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;
use crate::util;
use crate::util::d3d11_compile_bound_shader;

// todo: get rid of preset
type ShaderPassMeta<'a> = (
    &'a ShaderPassConfig,
    ShaderSource,
    CompilerBackend<
        impl CompileShader<HLSL, Options = Option<()>, Context = GlslangHlslContext> + ReflectShader,
    >,
);

pub struct FilterChain {
    pub common: FilterCommon,
    pub passes: Vec<FilterPass>,
    pub output_framebuffers: Box<[OwnedFramebuffer]>,
    pub feedback_framebuffers: Box<[OwnedFramebuffer]>,
    pub history_framebuffers: VecDeque<OwnedFramebuffer>,
    pub(crate) draw_quad: DrawQuad,
}

pub struct Direct3D11 {
    pub(crate) device: ID3D11Device,
    pub(crate) device_context: ID3D11DeviceContext,
}

pub struct FilterCommon {
    pub(crate) d3d11: Direct3D11,
    pub(crate) preset: ShaderPreset,
    pub(crate) luts: FxHashMap<usize, OwnedTexture>,
    pub samplers: SamplerSet,
    pub output_textures: Box<[Option<Texture>]>,
    pub feedback_textures: Box<[Option<Texture>]>,
    pub history_textures: Box<[Option<Texture>]>,
}

impl FilterChain {
    fn create_constant_buffer(device: &ID3D11Device, size: u32) -> util::Result<ID3D11Buffer> {
        eprintln!("{size}");
        unsafe {
            let buffer = device.CreateBuffer(&D3D11_BUFFER_DESC {
                ByteWidth: size,
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                MiscFlags: D3D11_RESOURCE_MISC_FLAG(0),
                StructureByteStride: 0,
            }, None)?;

            Ok(buffer)
        }
    }

    fn init_passes(
        device: &ID3D11Device,
        passes: Vec<ShaderPassMeta>,
        semantics: &ReflectSemantics,
    ) -> util::Result<Vec<FilterPass>>
    {
        // let mut filters = Vec::new();
        let mut filters = Vec::new();

        for (index, (config, source, mut reflect)) in passes.into_iter().enumerate() {
            let reflection = reflect.reflect(index, semantics)?;
            let hlsl = reflect.compile(None)?;

            let vertex_dxil = util::d3d_compile_shader(
                hlsl.vertex.as_bytes(),
                b"main\0",
                b"vs_5_0\0"
            )?;
            let vs = d3d11_compile_bound_shader(device, &vertex_dxil, None,
                                                ID3D11Device::CreateVertexShader)?;

            let ia_desc = DrawQuad::get_spirv_cross_vbo_desc();
            let vao = util::d3d11_create_input_layout(device, &ia_desc, &vertex_dxil)?;

            let fragment_dxil = util::d3d_compile_shader(
                hlsl.fragment.as_bytes(),
                b"main\0",
                b"ps_5_0\0"
            )?;
            let ps = d3d11_compile_bound_shader(device, &fragment_dxil, None,
                                                ID3D11Device::CreatePixelShader)?;


            let ubo_cbuffer = if let Some(ubo) = &reflection.ubo && ubo.size != 0 {
                let buffer = FilterChain::create_constant_buffer(device, ubo.size)?;
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
                let buffer = FilterChain::create_constant_buffer(device, push.size)?;
                Some(ConstantBufferBinding {
                    binding: if ubo_cbuffer.is_some() { 1 } else { 0 },
                    size: push.size,
                    stage_mask: push.stage_mask,
                    buffer,
                })
            } else {
                None
            };

            let uniform_storage = UniformStorage::new(reflection
                                                         .ubo
                                                         .as_ref()
                                                         .map(|ubo| ubo.size as usize)
                                                         .unwrap_or(0),
                                                     reflection
                                                         .push_constant
                                                         .as_ref()
                                                         .map(|push| push.size as usize)
                                                         .unwrap_or(0));

            let mut uniform_bindings = FxHashMap::default();
            for param in reflection.meta.parameter_meta.values() {
                uniform_bindings.insert(
                    UniformBinding::Parameter(param.id.clone()),
                    param.offset,
                );
            }

            for (semantics, param) in &reflection.meta.variable_meta {
                uniform_bindings.insert(
                    UniformBinding::SemanticVariable(*semantics),
                    param.offset
                );
            }

            for (semantics, param) in &reflection.meta.texture_size_meta {
                uniform_bindings.insert(
                    UniformBinding::TextureSize(*semantics),
                    param.offset
                );
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
    /// Load a filter chain from a pre-parsed `ShaderPreset`.
    pub fn load_from_preset(device: &ID3D11Device, preset: ShaderPreset) -> util::Result<FilterChain> {
        let (passes, semantics) = FilterChain::load_preset(&preset)?;

        let samplers = SamplerSet::new(device)?;

        // initialize passes
        let filters = FilterChain::init_passes(device, passes, &semantics).unwrap();

        let mut device_context = None;
        unsafe {
            device.GetImmediateContext(&mut device_context);
        }
        let device_context = device_context.unwrap();

        // initialize output framebuffers
        let mut output_framebuffers = Vec::new();
        output_framebuffers.resize_with(filters.len(), || OwnedFramebuffer::new(device, &device_context, Size::new(1, 1),
                                                                                ImageFormat::R8G8B8A8Unorm).unwrap());
        let mut output_textures = Vec::new();
        output_textures.resize_with(filters.len(), || None);
        //
        // // initialize feedback framebuffers
        let mut feedback_framebuffers = Vec::new();
        feedback_framebuffers.resize_with(filters.len(), || OwnedFramebuffer::new(device, &device_context, Size::new(1, 1),
                                                                                ImageFormat::R8G8B8A8Unorm).unwrap());
        let mut feedback_textures = Vec::new();
        feedback_textures.resize_with(filters.len(), || None);

        // load luts
        let luts = FilterChain::load_luts(device, &preset.textures)?;

        let (history_framebuffers, history_textures) =
            FilterChain::init_history(device, &device_context, &filters);


        let draw_quad = DrawQuad::new(device, &device_context)?;

        // todo: make vbo: d3d11.c 1376
        Ok(FilterChain {
            passes: filters,
            output_framebuffers: output_framebuffers.into_boxed_slice(),
            feedback_framebuffers: feedback_framebuffers.into_boxed_slice(),
            history_framebuffers,
            draw_quad,
            common: FilterCommon {
                d3d11: Direct3D11 {
                    device: device.clone(),
                    device_context
                },
                luts,
                samplers,
                // we don't need the reflect semantics once all locations have been bound per pass.
                // semantics,
                preset,
                output_textures: output_textures.into_boxed_slice(),
                feedback_textures: feedback_textures.into_boxed_slice(),
                history_textures,
            },
        })
    }


    fn init_history(
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        filters: &[FilterPass],
    ) -> (VecDeque<OwnedFramebuffer>, Box<[Option<Texture>]>) {
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
            println!("[history] not using frame history");
            return (VecDeque::new(), Box::new([]));
        }

        // history0 is aliased with the original

        eprintln!("[history] using frame history with {required_images} images");
        let mut framebuffers = VecDeque::with_capacity(required_images);
        framebuffers.resize_with(required_images, || OwnedFramebuffer::new(device, &context, Size::new(1, 1),
                                                                           ImageFormat::R8G8B8A8Unorm).unwrap());

        let mut history_textures = Vec::new();
        history_textures.resize_with(required_images, || None);

        (framebuffers, history_textures.into_boxed_slice())
    }

    fn push_history(&mut self, input: &DxImageView) -> util::Result<()> {
        if let Some(mut back) = self.history_framebuffers.pop_back() {
            let resource = unsafe {
                let mut resource = None;
                input.handle.GetResource(&mut resource);

                // todo: make panic-free
                resource.unwrap()
            };

            let format = unsafe {
                let mut desc = Default::default();
                input.handle.GetDesc(&mut desc);
                desc.Format
            };

            if back.size != input.size || (format != DXGI_FORMAT(0) && format != back.format) {
                eprintln!("[history] resizing");
                back.init(input.size, ImageFormat::from(format))?;
            }

            back.copy_from(&resource)?;

            self.history_framebuffers.push_front(back)
        }

        Ok(())
    }


    fn load_luts(
        device: &ID3D11Device,
        textures: &[TextureConfig],
    ) -> util::Result<FxHashMap<usize, OwnedTexture>> {
        let mut luts = FxHashMap::default();

        for (index, texture) in textures.iter().enumerate() {
            let image = Image::load(&texture.path)?;
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

            let mut texture = OwnedTexture::new(device, &image, desc,
                                                texture.filter_mode, texture.wrap_mode)?;
            luts.insert(index, texture);
        }
        Ok(luts)
    }

    /// Load the shader preset at the given path into a filter chain.
    pub fn load_from_path(device: &ID3D11Device, path: impl AsRef<Path>) -> util::Result<FilterChain> {
        // load passes from preset
        let preset = ShaderPreset::try_parse(path)?;
        Self::load_from_preset(device, preset)
    }

    fn load_preset(preset: &ShaderPreset) -> util::Result<(Vec<ShaderPassMeta>, ReflectSemantics)> {
        let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();
        let mut texture_semantics: FxHashMap<String, SemanticMap<TextureSemantics>> =
            Default::default();

        let passes = preset
            .shaders
            .iter()
            .map(|shader| {
                eprintln!("[dx11] loading {}", &shader.name.display());
                let source: ShaderSource = ShaderSource::load(&shader.name)?;

                let spirv = GlslangCompilation::compile(&source)?;
                let reflect = HLSL::from_compilation(spirv)?;

                for parameter in source.parameters.iter() {
                    uniform_semantics.insert(
                        parameter.id.clone(),
                        UniformSemantic::Variable(SemanticMap {
                            semantics: VariableSemantics::FloatParameter,
                            index: (),
                        }),
                    );
                }
                Ok::<_, Box<dyn Error>>((shader, source, reflect))
            })
            .into_iter()
            .collect::<util::Result<Vec<(&ShaderPassConfig, ShaderSource, CompilerBackend<_>)>>>()?;

        for details in &passes {
            librashader_runtime::semantics::insert_pass_semantics(
                &mut uniform_semantics,
                &mut texture_semantics,
                details.0,
            )
        }
        librashader_runtime::semantics::insert_lut_semantics(&preset.textures,
                                                             &mut uniform_semantics,
                                                             &mut texture_semantics);

        let semantics = ReflectSemantics {
            uniform_semantics,
            texture_semantics,
        };

        Ok((passes, semantics))
    }

    pub fn frame(&mut self, count: usize, viewport: &Size<u32>, input: DxImageView, output: OutputFramebuffer) -> util::Result<()> {

        let passes = &mut self.passes;

        if passes.is_empty() {
            return Ok(());
        }

        let filter = passes[0].config.filter;
        let wrap_mode = passes[0].config.wrap_mode;

        self.draw_quad.bind_vertices();

        let original = Texture {
            view: input.clone(),
            filter,
            wrap_mode,
        };

        let mut source = original.clone();

        // rescale render buffers to ensure all bindings are valid.
        for (index, pass) in passes.iter_mut().enumerate() {
            self.output_framebuffers[index].scale(
                pass.config.scaling.clone(),
                pass.get_format(),
                viewport,
                &original,
                &source,
            )?;

            self.feedback_framebuffers[index].scale(
                pass.config.scaling.clone(),
                pass.get_format(),
                viewport,
                &original,
                &source,
            )?;
        }

        let passes_len = passes.len();
        let (pass, last) = passes.split_at_mut(passes_len - 1);


        for (index, pass) in pass.iter_mut().enumerate() {
            let target = &self.output_framebuffers[index];
            let size =  target.size;

            pass.draw(index, &self.common, if pass.config.frame_count_mod > 0 {
                count % pass.config.frame_count_mod as usize
            } else {
                count
            } as u32, 1, viewport, &original, &source, RenderTarget::new(target.as_output_framebuffer().unwrap(), None))?;

            source = Texture {
                view: DxImageView { handle: target.create_shader_resource_view().unwrap(), size },
                filter,
                wrap_mode,
            };
            self.common.output_textures[index] = Some(source.clone());
        }

        assert_eq!(last.len(), 1);
        for pass in last {
            source.filter = pass.config.filter;
            pass.draw(
                passes_len - 1,
                &self.common,
                if pass.config.frame_count_mod > 0 {
                    count % pass.config.frame_count_mod as usize
                } else {
                    count
                } as u32,
                1, viewport, &original, &source, RenderTarget::new(output, None))?;

            // diverge so we don't need to clone output.
            break;
        }

        // swap feedback framebuffers with output
        for (output, feedback) in self
            .output_framebuffers
            .iter_mut()
            .zip(self.feedback_framebuffers.iter_mut())
        {
            std::mem::swap(output, feedback);
        }

        self.push_history(&input)?;
        Ok(())

    }
}
