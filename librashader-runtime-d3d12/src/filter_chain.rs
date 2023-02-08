use crate::buffer::{D3D12Buffer, RawD3D12Buffer};
use crate::descriptor_heap::{
    CpuStagingHeap, D3D12DescriptorHeap, RenderTargetHeap, ResourceWorkHeap,
};
use crate::error::FilterChainError;
use crate::filter_pass::FilterPass;
use crate::framebuffer::OwnedImage;
use crate::graphics_pipeline::{D3D12GraphicsPipeline, D3D12RootSignature};
use crate::luts::LutTexture;
use crate::mipmap::D3D12MipmapGen;
use crate::options::{FilterChainOptionsD3D12, FrameOptionsD3D12};
use crate::quad_render::DrawQuad;
use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;
use crate::texture::{D3D12InputImage, D3D12OutputView, InputTexture, OutputDescriptor};
use crate::{error, util};
use librashader_common::{ImageFormat, Size, Viewport};
use librashader_presets::{ShaderPreset, TextureConfig};
use librashader_reflect::back::targets::{DXIL, HLSL};
use librashader_reflect::back::{CompileReflectShader, CompileShader};
use librashader_reflect::front::GlslangCompilation;
use librashader_reflect::reflect::presets::{CompilePresetTarget, ShaderPassArtifact};
use librashader_reflect::reflect::semantics::{BindingMeta, ShaderSemantics, MAX_BINDINGS_COUNT};
use librashader_reflect::reflect::ReflectShader;
use librashader_runtime::binding::{BindingUtil, TextureInput};
use librashader_runtime::image::{Image, UVDirection};
use librashader_runtime::quad::{QuadType, DEFAULT_MVP, IDENTITY_MVP};
use librashader_runtime::uniforms::UniformStorage;
use rustc_hash::FxHashMap;
use spirv_cross::hlsl::ShaderModel;
use std::collections::VecDeque;
use std::path::Path;
use windows::core::Interface;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Graphics::Direct3D::Dxc::{
    CLSID_DxcCompiler, CLSID_DxcLibrary, CLSID_DxcValidator, DxcCreateInstance, IDxcCompiler,
    IDxcUtils, IDxcValidator,
};
use windows::Win32::Graphics::Direct3D12::{
    ID3D12CommandAllocator, ID3D12CommandQueue, ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence,
    ID3D12GraphicsCommandList, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
    D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_FENCE_FLAG_NONE,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_RENDER_TARGET,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN;
use windows::Win32::System::Threading::{CreateEventA, ResetEvent, WaitForSingleObject};
use windows::Win32::System::WindowsProgramming::INFINITE;

use librashader_runtime::scaling::{MipmapSize, ScaleFramebuffer};
use rayon::prelude::*;

const MIPMAP_RESERVED_WORKHEAP_DESCRIPTORS: usize = 1024;

type DxilShaderPassMeta =
    ShaderPassArtifact<impl CompileReflectShader<DXIL, GlslangCompilation> + Send>;
type HlslShaderPassMeta =
    ShaderPassArtifact<impl CompileReflectShader<HLSL, GlslangCompilation> + Send>;

pub struct FilterMutable {
    pub(crate) passes_enabled: usize,
    pub(crate) parameters: FxHashMap<String, f32>,
}

/// A Direct3D 12 filter chain.
pub struct FilterChainD3D12 {
    pub(crate) common: FilterCommon,
    pub(crate) passes: Vec<FilterPass>,
    pub(crate) output_framebuffers: Box<[OwnedImage]>,
    pub(crate) feedback_framebuffers: Box<[OwnedImage]>,
    pub(crate) history_framebuffers: VecDeque<OwnedImage>,
    staging_heap: D3D12DescriptorHeap<CpuStagingHeap>,
    rtv_heap: D3D12DescriptorHeap<RenderTargetHeap>,

    work_heap: ID3D12DescriptorHeap,
    sampler_heap: ID3D12DescriptorHeap,

    residuals: Vec<OutputDescriptor>,
    mipmap_heap: D3D12DescriptorHeap<ResourceWorkHeap>,

    disable_mipmaps: bool,
}

pub(crate) struct FilterCommon {
    pub(crate) d3d12: ID3D12Device,
    pub samplers: SamplerSet,
    pub output_textures: Box<[Option<InputTexture>]>,
    pub feedback_textures: Box<[Option<InputTexture>]>,
    pub history_textures: Box<[Option<InputTexture>]>,
    pub config: FilterMutable,
    // pub disable_mipmaps: bool,
    pub luts: FxHashMap<usize, LutTexture>,
    pub mipmap_gen: D3D12MipmapGen,
    pub root_signature: D3D12RootSignature,
    pub draw_quad: DrawQuad,
}

impl FilterChainD3D12 {
    /// Load the shader preset at the given path into a filter chain.
    pub fn load_from_path(
        device: &ID3D12Device,
        path: impl AsRef<Path>,
        options: Option<&FilterChainOptionsD3D12>,
    ) -> error::Result<FilterChainD3D12> {
        // load passes from preset
        let preset = ShaderPreset::try_parse(path)?;
        Self::load_from_preset(device, preset, options)
    }

    /// Load a filter chain from a pre-parsed `ShaderPreset`.
    pub fn load_from_preset(
        device: &ID3D12Device,
        preset: ShaderPreset,
        options: Option<&FilterChainOptionsD3D12>,
    ) -> error::Result<FilterChainD3D12> {
        let shader_count = preset.shaders.len();
        let lut_count = preset.textures.len();

        let shader_copy = preset.shaders.clone();

        let (passes, semantics) = DXIL::compile_preset_passes::<
            GlslangCompilation,
            FilterChainError,
        >(preset.shaders, &preset.textures)?;

        let (hlsl_passes, _) = HLSL::compile_preset_passes::<GlslangCompilation, FilterChainError>(
            shader_copy,
            &preset.textures,
        )?;

        let samplers = SamplerSet::new(device)?;
        let mipmap_gen = D3D12MipmapGen::new(device, false)?;

        let draw_quad = DrawQuad::new(device)?;
        let mut staging_heap = D3D12DescriptorHeap::new(
            device,
            (MAX_BINDINGS_COUNT as usize) * shader_count + 2048 + lut_count,
        )?;
        let rtv_heap = D3D12DescriptorHeap::new(
            device,
            (MAX_BINDINGS_COUNT as usize) * shader_count + 2048 + lut_count,
        )?;

        let luts = FilterChainD3D12::load_luts(device, &mut staging_heap, &preset.textures)?;

        let root_signature = D3D12RootSignature::new(device)?;

        let (texture_heap, sampler_heap, filters, mipmap_heap) = FilterChainD3D12::init_passes(
            device,
            &root_signature,
            passes,
            hlsl_passes,
            &semantics,
            options.map_or(false, |o| o.force_hlsl_pipeline),
        )?;

        // initialize output framebuffers
        let mut output_framebuffers = Vec::new();
        output_framebuffers.resize_with(filters.len(), || {
            OwnedImage::new(device, Size::new(1, 1), ImageFormat::R8G8B8A8Unorm, false)
        });

        // resolve all results
        let output_framebuffers = output_framebuffers
            .into_iter()
            .collect::<error::Result<Vec<OwnedImage>>>()?;
        let mut output_textures = Vec::new();
        output_textures.resize_with(filters.len(), || None);

        // let mut output_textures = Vec::new();
        // output_textures.resize_with(filters.len(), || None);
        //
        // // initialize feedback framebuffers
        let mut feedback_framebuffers = Vec::new();
        feedback_framebuffers.resize_with(filters.len(), || {
            OwnedImage::new(device, Size::new(1, 1), ImageFormat::R8G8B8A8Unorm, false)
        });

        // resolve all results
        let feedback_framebuffers = feedback_framebuffers
            .into_iter()
            .collect::<error::Result<Vec<OwnedImage>>>()?;
        let mut feedback_textures = Vec::new();
        feedback_textures.resize_with(filters.len(), || None);

        let (history_framebuffers, history_textures) =
            FilterChainD3D12::init_history(device, &filters)?;

        Ok(FilterChainD3D12 {
            common: FilterCommon {
                d3d12: device.clone(),
                samplers,
                output_textures: output_textures.into_boxed_slice(),
                feedback_textures: feedback_textures.into_boxed_slice(),
                luts,
                mipmap_gen,
                root_signature,
                draw_quad,
                config: FilterMutable {
                    passes_enabled: preset.shader_count as usize,
                    parameters: preset
                        .parameters
                        .into_iter()
                        .map(|param| (param.name, param.value))
                        .collect(),
                },
                history_textures,
            },
            staging_heap,
            rtv_heap,
            passes: filters,
            output_framebuffers: output_framebuffers.into_boxed_slice(),
            feedback_framebuffers: feedback_framebuffers.into_boxed_slice(),
            history_framebuffers,
            work_heap: texture_heap,
            sampler_heap,
            mipmap_heap,
            disable_mipmaps: options.map_or(false, |o| o.force_no_mipmaps),
            residuals: Vec::new(),
        })
    }

    fn init_history(
        device: &ID3D12Device,
        filters: &Vec<FilterPass>,
    ) -> error::Result<(VecDeque<OwnedImage>, Box<[Option<InputTexture>]>)> {
        let required_images =
            BindingMeta::calculate_required_history(filters.iter().map(|f| &f.reflection.meta));

        // not using frame history;
        if required_images <= 1 {
            // println!("[history] not using frame history");
            return Ok((VecDeque::new(), Box::new([])));
        }

        // history0 is aliased with the original

        // eprintln!("[history] using frame history with {required_images} images");
        let mut framebuffers = VecDeque::with_capacity(required_images);
        framebuffers.resize_with(required_images, || {
            OwnedImage::new(device, Size::new(1, 1), ImageFormat::R8G8B8A8Unorm, false)
        });

        let framebuffers = framebuffers
            .into_iter()
            .collect::<error::Result<VecDeque<OwnedImage>>>()?;

        let mut history_textures = Vec::new();
        history_textures.resize_with(required_images, || None);

        Ok((framebuffers, history_textures.into_boxed_slice()))
    }

    fn load_luts(
        device: &ID3D12Device,
        heap: &mut D3D12DescriptorHeap<CpuStagingHeap>,
        textures: &[TextureConfig],
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
        // use separate mipgen to load luts.
        let mipmap_gen = D3D12MipmapGen::new(device, true)?;
        let mut work_heap: D3D12DescriptorHeap<ResourceWorkHeap> =
            D3D12DescriptorHeap::new(device, u16::MAX as usize)?;
        unsafe {
            // 1 time queue infrastructure for lut uploads
            let command_pool: ID3D12CommandAllocator =
                device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)?;
            let cmd: ID3D12GraphicsCommandList =
                device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &command_pool, None)?;
            let queue: ID3D12CommandQueue =
                device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                    Priority: 0,
                    Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                    NodeMask: 0,
                })?;

            let fence_event = unsafe { CreateEventA(None, false, false, None)? };
            let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE)?;
            let mut residuals = Vec::new();

            let mut luts = FxHashMap::default();

            for (index, texture) in textures.iter().enumerate() {
                let image = Image::load(&texture.path, UVDirection::TopLeft)?;

                let (texture, staging) = LutTexture::new(
                    device,
                    heap,
                    &cmd,
                    &image,
                    texture.filter_mode,
                    texture.wrap_mode,
                    texture.mipmap,
                )?;
                luts.insert(index, texture);
                residuals.push(staging);
            }

            cmd.Close()?;

            queue.ExecuteCommandLists(&[cmd.cast()?]);
            queue.Signal(&fence, 1)?;

            // Wait until finished
            if unsafe { fence.GetCompletedValue() } < 1 {
                unsafe { fence.SetEventOnCompletion(1, fence_event)? };

                unsafe { WaitForSingleObject(fence_event, INFINITE) };
                unsafe { ResetEvent(fence_event) };
            }

            cmd.Reset(&command_pool, None)?;

            let residuals = mipmap_gen.mipmapping_context(&cmd, &mut work_heap, |context| {
                for lut in luts.values() {
                    lut.generate_mipmaps(context)?;
                }

                Ok::<(), FilterChainError>(())
            })?;

            //
            cmd.Close()?;
            queue.ExecuteCommandLists(&[cmd.cast()?]);
            queue.Signal(&fence, 2)?;
            //
            if unsafe { fence.GetCompletedValue() } < 2 {
                unsafe { fence.SetEventOnCompletion(2, fence_event)? }

                unsafe { WaitForSingleObject(fence_event, INFINITE) };
                unsafe { CloseHandle(fence_event) };
            }

            drop(residuals);
            Ok(luts)
        }
    }

    fn init_passes(
        device: &ID3D12Device,
        root_signature: &D3D12RootSignature,
        passes: Vec<DxilShaderPassMeta>,
        hlsl_passes: Vec<HlslShaderPassMeta>,
        semantics: &ShaderSemantics,
        force_hlsl: bool,
    ) -> error::Result<(
        ID3D12DescriptorHeap,
        ID3D12DescriptorHeap,
        Vec<FilterPass>,
        D3D12DescriptorHeap<ResourceWorkHeap>,
    )> {
        let shader_count = passes.len();
        let work_heap = D3D12DescriptorHeap::<ResourceWorkHeap>::new(
            device,
            (MAX_BINDINGS_COUNT as usize) * shader_count + MIPMAP_RESERVED_WORKHEAP_DESCRIPTORS,
        )?;
        let (work_heaps, mipmap_heap, texture_heap_handle) = unsafe {
            work_heap.suballocate(
                MAX_BINDINGS_COUNT as usize,
                MIPMAP_RESERVED_WORKHEAP_DESCRIPTORS,
            )
        };

        let sampler_work_heap =
            D3D12DescriptorHeap::new(device, (MAX_BINDINGS_COUNT as usize) * shader_count)?;

        let (sampler_work_heaps, _, sampler_heap_handle) =
            unsafe { sampler_work_heap.suballocate(MAX_BINDINGS_COUNT as usize, 0) };

        let filters: Vec<error::Result<_>> = passes.into_par_iter()
            .zip(hlsl_passes)
            .enumerate()
            .map_init(
                || {
                    let validator: IDxcValidator = unsafe { DxcCreateInstance(&CLSID_DxcValidator)? };
                    let library: IDxcUtils = unsafe { DxcCreateInstance(&CLSID_DxcLibrary)? };
                    let compiler: IDxcCompiler = unsafe { DxcCreateInstance(&CLSID_DxcCompiler)? };
                    Ok::<_, FilterChainError>((validator, library, compiler))
                },
                |dxc, (index, ((config, source, mut dxil),
                          (_, _, mut hlsl)))| {
                let Ok((validator, library, compiler)) = dxc else {
                    return Err(FilterChainError::Direct3DOperationError("Could not initialize DXC for thread"));
                };

                let dxil_reflection = dxil.reflect(index, semantics)?;
                let dxil = dxil.compile(Some(
                    librashader_reflect::back::dxil::ShaderModel::ShaderModel6_0,
                ))?;

                let hlsl_reflection = hlsl.reflect(index, semantics)?;
                let hlsl = hlsl.compile(Some(ShaderModel::V6_0))?;

                let render_format = if let Some(format) = config.get_format_override() {
                    format
                } else if source.format != ImageFormat::Unknown {
                    source.format
                } else {
                    ImageFormat::R8G8B8A8Unorm
                }.into();

                // incredibly cursed.
                let (reflection, graphics_pipeline) = if !force_hlsl &&
                    let Ok(graphics_pipeline) =
                        D3D12GraphicsPipeline::new_from_dxil(
                            device,
                            library,
                            validator,
                            &dxil,
                            root_signature,
                            render_format,
                        ) {
                    (dxil_reflection, graphics_pipeline)
                } else {
                    let graphics_pipeline = D3D12GraphicsPipeline::new_from_hlsl(
                        device,
                        library,
                        compiler,
                        &hlsl,
                        root_signature,
                        render_format,
                    )?;
                    (hlsl_reflection, graphics_pipeline)
                };

                    // minimum size here has to be 1 byte.
                    let ubo_size = reflection.ubo.as_ref().map_or(1, |ubo| ubo.size as usize);
                    let push_size = reflection
                        .push_constant
                        .as_ref()
                        .map_or(1, |push| push.size as usize);

                    let uniform_storage = UniformStorage::new_with_storage(
                    RawD3D12Buffer::new(D3D12Buffer::new(device, ubo_size)?)?,
                    RawD3D12Buffer::new(D3D12Buffer::new(device, push_size)?)?
                );


                let uniform_bindings = reflection.meta.create_binding_map(|param| param.offset());

                Ok((reflection,
                    uniform_bindings,
                    uniform_storage,
                    graphics_pipeline,
                    config,
                    source))

            }).collect();

        let filters: error::Result<Vec<_>> = filters.into_iter().collect();
        let filters = filters?;

        // Need to take care of the heaps in a single thread because [;16] is not sized..?
        let filters: Vec<error::Result<FilterPass>> = filters
            .into_iter()
            .zip(work_heaps)
            .zip(sampler_work_heaps)
            .map(
                |(
                    (
                        (reflection, uniform_bindings, uniform_storage, pipeline, config, source),
                        mut texture_heap,
                    ),
                    mut sampler_heap,
                )| {
                    let texture_heap = texture_heap.alloc_range()?;
                    let sampler_heap = sampler_heap.alloc_range()?;
                    Ok(FilterPass {
                        reflection,
                        uniform_bindings,
                        uniform_storage,
                        pipeline,
                        config,
                        texture_heap,
                        sampler_heap,
                        source,
                    })
                },
            )
            .collect();
        let filters: error::Result<Vec<_>> = filters.into_iter().collect();
        let filters = filters?;

        // Panic SAFETY: mipmap_heap is always 1024 descriptors.
        Ok((
            texture_heap_handle,
            sampler_heap_handle,
            filters,
            mipmap_heap.unwrap(),
        ))
    }

    fn push_history(
        &mut self,
        cmd: &ID3D12GraphicsCommandList,
        input: &InputTexture,
    ) -> error::Result<()> {
        if let Some(mut back) = self.history_framebuffers.pop_back() {
            if back.size != input.size
                || (input.format != DXGI_FORMAT_UNKNOWN && input.format != back.format.into())
            {
                // eprintln!("[history] resizing");
                // old back will get dropped.. do we need to defer?
                let _old_back = std::mem::replace(
                    &mut back,
                    OwnedImage::new(&self.common.d3d12, input.size, input.format.into(), false)?,
                );
            }
            unsafe {
                back.copy_from(cmd, input)?;
            }
            self.history_framebuffers.push_front(back);
        }

        Ok(())
    }

    /// Records shader rendering commands to the provided command list.
    ///
    /// * The input image must be in the `D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE` resource state.
    /// * The output image must be in `D3D12_RESOURCE_STATE_RENDER_TARGET` resource state.
    ///
    /// librashader **will not** create a resource barrier for the final pass. The output image will
    /// remain in `D3D12_RESOURCE_STATE_RENDER_TARGET` after all shader passes. The caller must transition
    /// the output image to the final resource state.
    pub fn frame(
        &mut self,
        cmd: &ID3D12GraphicsCommandList,
        input: D3D12InputImage,
        viewport: &Viewport<D3D12OutputView>,
        frame_count: usize,
        options: Option<&FrameOptionsD3D12>,
    ) -> error::Result<()> {
        self.residuals.clear();

        if let Some(options) = options {
            if options.clear_history {
                for framebuffer in &mut self.history_framebuffers {
                    framebuffer.clear(cmd, &mut self.rtv_heap)?;
                }
            }
        }

        // limit number of passes to those enabled.
        let max = std::cmp::min(self.passes.len(), self.common.config.passes_enabled);
        let passes = &mut self.passes[0..max];

        if passes.is_empty() {
            return Ok(());
        }

        let frame_direction = options.map_or(1, |f| f.frame_direction);

        let max = std::cmp::min(self.passes.len(), self.common.config.passes_enabled);
        let passes = &mut self.passes[0..max];
        if passes.is_empty() {
            return Ok(());
        }

        let filter = passes[0].config.filter;
        let wrap_mode = passes[0].config.wrap_mode;

        for ((texture, fbo), pass) in self
            .common
            .feedback_textures
            .iter_mut()
            .zip(self.feedback_framebuffers.iter())
            .zip(passes.iter())
        {
            *texture = Some(fbo.create_shader_resource_view(
                &mut self.staging_heap,
                pass.config.filter,
                pass.config.wrap_mode,
            )?);
        }

        for (texture, fbo) in self
            .common
            .history_textures
            .iter_mut()
            .zip(self.history_framebuffers.iter())
        {
            *texture =
                Some(fbo.create_shader_resource_view(&mut self.staging_heap, filter, wrap_mode)?);
        }

        let original = unsafe { InputTexture::new_from_raw(input, filter, wrap_mode) };
        let mut source = original.clone();

        // swap output and feedback **before** recording command buffers
        std::mem::swap(
            &mut self.output_framebuffers,
            &mut self.feedback_framebuffers,
        );

        // rescale render buffers to ensure all bindings are valid.
        OwnedImage::scale_framebuffers(
            source.size(),
            viewport.output.size,
            &mut self.output_framebuffers,
            &mut self.feedback_framebuffers,
            passes,
            Some(&mut |index, pass, output, feedback| {
                // refresh inputs
                self.common.feedback_textures[index] = Some(feedback.create_shader_resource_view(
                    &mut self.staging_heap,
                    pass.config.filter,
                    pass.config.wrap_mode,
                )?);
                self.common.output_textures[index] = Some(output.create_shader_resource_view(
                    &mut self.staging_heap,
                    pass.config.filter,
                    pass.config.wrap_mode,
                )?);

                Ok(())
            }),
        )?;

        let passes_len = passes.len();
        let (pass, last) = passes.split_at_mut(passes_len - 1);

        unsafe {
            let heaps = [self.work_heap.clone(), self.sampler_heap.clone()];
            cmd.SetDescriptorHeaps(&heaps);
            cmd.SetGraphicsRootSignature(&self.common.root_signature.handle);
            self.common.mipmap_gen.pin_root_signature(cmd);
        }

        self.common.draw_quad.bind_vertices_for_frame(cmd);

        for (index, pass) in pass.iter_mut().enumerate() {
            source.filter = pass.config.filter;
            source.wrap_mode = pass.config.wrap_mode;

            if pass.config.mipmap_input && !self.disable_mipmaps {
                unsafe {
                    self.common.mipmap_gen.mipmapping_context(
                        cmd,
                        &mut self.mipmap_heap,
                        |ctx| {
                            ctx.generate_mipmaps(
                                &source.resource,
                                source.size().calculate_miplevels() as u16,
                                source.size,
                                source.format,
                            )?;
                            Ok::<(), FilterChainError>(())
                        },
                    )?;
                }
            }

            let target = &self.output_framebuffers[index];
            util::d3d12_resource_transition(
                cmd,
                &target.handle,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
            );
            let size = target.size;
            let view = target.create_render_target_view(&mut self.rtv_heap)?;

            let out = RenderTarget {
                x: 0.0,
                y: 0.0,
                mvp: IDENTITY_MVP,
                output: D3D12OutputView {
                    descriptor: view.descriptor,
                    size,
                },
            };

            pass.draw(
                cmd,
                index,
                &self.common,
                pass.config.get_frame_count(frame_count),
                frame_direction,
                viewport,
                &original,
                &source,
                &out,
                QuadType::Offscreen,
            )?;

            util::d3d12_resource_transition(
                cmd,
                &target.handle,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            );

            self.residuals.push(out.output.descriptor);
            source = self.common.output_textures[index].as_ref().unwrap().clone()
        }

        // try to hint the optimizer
        assert_eq!(last.len(), 1);
        if let Some(pass) = last.iter_mut().next() {
            source.filter = pass.config.filter;
            source.wrap_mode = pass.config.wrap_mode;

            let out = RenderTarget {
                x: 0.0,
                y: 0.0,
                mvp: DEFAULT_MVP,
                output: viewport.output.clone(),
            };

            pass.draw(
                cmd,
                passes_len - 1,
                &self.common,
                pass.config.get_frame_count(frame_count),
                frame_direction,
                viewport,
                &original,
                &source,
                &out,
                QuadType::Final,
            )?;
        }

        self.push_history(cmd, &original)?;

        Ok(())
    }
}
