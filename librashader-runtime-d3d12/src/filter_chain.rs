use crate::buffer::{D3D12Buffer, D3D12ConstantBuffer};
use crate::descriptor_heap::{
    CpuStagingHeap, D3D12DescriptorHeap, RenderTargetHeap, ResourceWorkHeap,
};
use crate::filter_pass::FilterPass;
use crate::framebuffer::OwnedImage;
use crate::graphics_pipeline::{D3D12GraphicsPipeline, D3D12RootSignature};
use crate::luts::LutTexture;
use crate::mipmap::D3D12MipmapGen;
use crate::options::FilterChainOptionsD3D12;
use crate::quad_render::DrawQuad;
use crate::render_target::RenderTarget;
use crate::samplers::SamplerSet;
use crate::texture::{InputTexture, OutputDescriptor, OutputTexture};
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
use librashader_runtime::quad::{QuadType, DEFAULT_MVP};
use librashader_runtime::scaling::MipmapSize;
use librashader_runtime::uniforms::UniformStorage;
use rustc_hash::FxHashMap;
use spirv_cross::hlsl::ShaderModel;
use std::collections::VecDeque;
use std::error::Error;
use std::path::Path;
use windows::core::Interface;
use windows::w;
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

type DxilShaderPassMeta = ShaderPassArtifact<impl CompileReflectShader<DXIL, GlslangCompilation>>;
type HlslShaderPassMeta = ShaderPassArtifact<impl CompileReflectShader<HLSL, GlslangCompilation>>;

pub struct FilterMutable {
    pub(crate) passes_enabled: usize,
    pub(crate) parameters: FxHashMap<String, f32>,
}

pub struct FilterChainD3D12 {
    pub(crate) common: FilterCommon,
    pub(crate) passes: Vec<FilterPass>,
    pub(crate) output_framebuffers: Box<[OwnedImage]>,
    pub(crate) feedback_framebuffers: Box<[OwnedImage]>,
    pub(crate) history_framebuffers: VecDeque<OwnedImage>,
    staging_heap: D3D12DescriptorHeap<CpuStagingHeap>,
    rtv_heap: D3D12DescriptorHeap<RenderTargetHeap>,

    texture_heap: ID3D12DescriptorHeap,
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

        let (passes, semantics) =
            DXIL::compile_preset_passes::<GlslangCompilation, Box<dyn Error>>(
                preset.shaders,
                &preset.textures,
            )
            .unwrap();

        let (hlsl_passes, _) = HLSL::compile_preset_passes::<GlslangCompilation, Box<dyn Error>>(
            shader_copy,
            &preset.textures,
        )
        .unwrap();

        let samplers = SamplerSet::new(device)?;
        let mipmap_gen = D3D12MipmapGen::new(device).unwrap();

        let draw_quad = DrawQuad::new(device)?;
        let mut staging_heap = D3D12DescriptorHeap::new(
            device,
            (MAX_BINDINGS_COUNT as usize) * shader_count + 2048 + lut_count,
        )?;
        let rtv_heap = D3D12DescriptorHeap::new(
            device,
            (MAX_BINDINGS_COUNT as usize) * shader_count + 2048 + lut_count,
        )?;

        let luts =
            FilterChainD3D12::load_luts(device, &mut staging_heap, &preset.textures, &mipmap_gen)
                .unwrap();

        let root_signature = D3D12RootSignature::new(device)?;

        let (texture_heap, sampler_heap, filters) = FilterChainD3D12::init_passes(
            device,
            &root_signature,
            passes,
            hlsl_passes,
            &semantics,
            options.map_or(false, |o| o.force_hlsl_pipeline),
        )
        .unwrap();

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

        let mipmap_heap: D3D12DescriptorHeap<ResourceWorkHeap> =
            D3D12DescriptorHeap::new(device, u16::MAX as usize)?;
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
            texture_heap,
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
        mipmap_gen: &D3D12MipmapGen,
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
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

            queue.SetName(w!("LutQueue"))?;

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
                unsafe { fence.SetEventOnCompletion(1, fence_event) }
                    .ok()
                    .unwrap();

                unsafe { WaitForSingleObject(fence_event, INFINITE) };
                unsafe { ResetEvent(fence_event) };
            }

            cmd.Reset(&command_pool, None).unwrap();

            let residuals = mipmap_gen.mipmapping_context(&cmd, &mut work_heap, |context| {
                for lut in luts.values() {
                    lut.generate_mipmaps(context)?;
                }

                Ok::<(), Box<dyn Error>>(())
            })?;

            //
            cmd.Close()?;
            queue.ExecuteCommandLists(&[cmd.cast()?]);
            queue.Signal(&fence, 2)?;
            //
            if unsafe { fence.GetCompletedValue() } < 2 {
                unsafe { fence.SetEventOnCompletion(2, fence_event) }
                    .ok()
                    .unwrap();

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
    ) -> error::Result<(ID3D12DescriptorHeap, ID3D12DescriptorHeap, Vec<FilterPass>)> {
        let validator: IDxcValidator = unsafe { DxcCreateInstance(&CLSID_DxcValidator)? };

        let library: IDxcUtils = unsafe { DxcCreateInstance(&CLSID_DxcLibrary)? };

        let compiler: IDxcCompiler = unsafe { DxcCreateInstance(&CLSID_DxcCompiler)? };

        let mut filters = Vec::new();
        let shader_count = passes.len();
        let work_heap = D3D12DescriptorHeap::<ResourceWorkHeap>::new(
            device,
            (MAX_BINDINGS_COUNT as usize) * shader_count,
        )?;
        let (work_heaps, texture_heap_handle) =
            unsafe { work_heap.suballocate(MAX_BINDINGS_COUNT as usize) };

        let sampler_work_heap =
            D3D12DescriptorHeap::new(device, (MAX_BINDINGS_COUNT as usize) * shader_count)?;

        let (sampler_work_heaps, sampler_heap_handle) =
            unsafe { sampler_work_heap.suballocate(MAX_BINDINGS_COUNT as usize) };

        for (
            index,
            ((((config, source, mut dxil), (_, _, mut hlsl)), mut texture_heap), mut sampler_heap),
        ) in passes
            .into_iter()
            .zip(hlsl_passes)
            .zip(work_heaps)
            .zip(sampler_work_heaps)
            .enumerate()
        {
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
            }
            .into();

            eprintln!("building pipeline for pass {index:?}");

            /// incredibly cursed.
            let (reflection, graphics_pipeline) = if !force_hlsl &&
                let Ok(graphics_pipeline) =
                D3D12GraphicsPipeline::new_from_dxil(
                    device,
                    &library,
                    &validator,
                    &dxil,
                    root_signature,
                    render_format,
                ) {
                (dxil_reflection, graphics_pipeline)
            } else {
                eprintln!("falling back to hlsl for {index:?}");
                let graphics_pipeline = D3D12GraphicsPipeline::new_from_hlsl(
                    device,
                    &library,
                    &compiler,
                    &hlsl,
                    root_signature,
                    render_format,
                )?;
                (hlsl_reflection, graphics_pipeline)
            };

            let uniform_storage = UniformStorage::new(
                reflection.ubo.as_ref().map_or(0, |ubo| ubo.size as usize),
                reflection
                    .push_constant
                    .as_ref()
                    .map_or(0, |push| push.size as usize),
            );

            let ubo_cbuffer = if let Some(ubo) = &reflection.ubo && ubo.size != 0 {
                let buffer = D3D12ConstantBuffer::new(D3D12Buffer::new(device, ubo.size as usize)?);
                Some(buffer)
            } else {
                None
            };

            let push_cbuffer = if let Some(push) = &reflection.push_constant && push.size != 0 {
                let buffer = D3D12ConstantBuffer::new(D3D12Buffer::new(device, push.size as usize)?);
                Some(buffer)
            } else {
                None
            };

            let uniform_bindings = reflection.meta.create_binding_map(|param| param.offset());

            let texture_heap = texture_heap.alloc_range()?;
            let sampler_heap = sampler_heap.alloc_range()?;
            filters.push(FilterPass {
                reflection,
                uniform_bindings,
                uniform_storage,
                push_cbuffer,
                ubo_cbuffer,
                pipeline: graphics_pipeline,
                config: config.clone(),
                texture_heap,
                sampler_heap,
                source,
            })
        }

        Ok((texture_heap_handle, sampler_heap_handle, filters))
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

    /// Process a frame with the input image.
    pub fn frame(
        &mut self,
        cmd: &ID3D12GraphicsCommandList,
        input: InputTexture,
        viewport: &Viewport<OutputTexture>,
        frame_count: usize,
        _options: Option<&()>,
    ) -> error::Result<()> {
        drop(self.residuals.drain(..));

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

        let original = input;
        let mut source = unsafe { original.clone() };

        // swap output and feedback **before** recording command buffers
        std::mem::swap(
            &mut self.output_framebuffers,
            &mut self.feedback_framebuffers,
        );

        // rescale render buffers to ensure all bindings are valid.
        let mut source_size = source.size();
        let mut iterator = passes.iter_mut().enumerate().peekable();
        while let Some((index, pass)) = iterator.next() {
            let should_mipmap = iterator
                .peek()
                .map_or(false, |(_, p)| p.config.mipmap_input);

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

            // refresh inputs
            self.common.feedback_textures[index] = Some(
                self.feedback_framebuffers[index].create_shader_resource_view(
                    &mut self.staging_heap,
                    pass.config.filter,
                    pass.config.wrap_mode,
                )?,
            );
            self.common.output_textures[index] =
                Some(self.output_framebuffers[index].create_shader_resource_view(
                    &mut self.staging_heap,
                    pass.config.filter,
                    pass.config.wrap_mode,
                )?);
        }

        let passes_len = passes.len();
        let (pass, last) = passes.split_at_mut(passes_len - 1);

        unsafe {
            let heaps = [self.texture_heap.clone(), self.sampler_heap.clone()];
            cmd.SetDescriptorHeaps(&heaps);
            cmd.SetGraphicsRootSignature(&self.common.root_signature.handle);
        }
        for (index, pass) in pass.iter_mut().enumerate() {
            source.filter = pass.config.filter;
            source.wrap_mode = pass.config.wrap_mode;

            if pass.config.mipmap_input && !self.disable_mipmaps {
                unsafe {
                    // this is so bad.
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
                            Ok::<(), Box<dyn Error>>(())
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
                mvp: DEFAULT_MVP,
                output: OutputTexture {
                    descriptor: view.descriptor,
                    size,
                },
            };

            pass.draw(
                cmd,
                index,
                &self.common,
                if pass.config.frame_count_mod > 0 {
                    frame_count % pass.config.frame_count_mod as usize
                } else {
                    frame_count
                } as u32,
                1,
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

            // let target_handle = target.create_shader_resource_view(
            //     &mut self.staging_heap,
            //     pass.config.filter,
            //     pass.config.wrap_mode,
            // )?;
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
                if pass.config.frame_count_mod > 0 {
                    frame_count % pass.config.frame_count_mod as usize
                } else {
                    frame_count
                } as u32,
                1,
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
