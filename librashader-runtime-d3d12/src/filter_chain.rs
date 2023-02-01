use std::collections::VecDeque;
use crate::{error};
use crate::heap::{D3D12DescriptorHeap, CpuStagingHeap, ResourceWorkHeap, SamplerWorkHeap};
use crate::samplers::SamplerSet;
use crate::luts::LutTexture;
use librashader_presets::{ShaderPreset, TextureConfig};
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::front::GlslangCompilation;
use librashader_reflect::reflect::presets::{CompilePresetTarget, ShaderPassArtifact};
use librashader_runtime::image::{Image, UVDirection};
use rustc_hash::FxHashMap;
use std::error::Error;
use std::path::Path;
use windows::core::Interface;
use windows::w;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Graphics::Direct3D12::{
    ID3D12CommandAllocator, ID3D12CommandQueue, ID3D12Device, ID3D12Fence,
    ID3D12GraphicsCommandList, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
    D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_FENCE_FLAG_NONE,
};
use windows::Win32::System::Threading::{CreateEventA, ResetEvent, WaitForSingleObject};
use windows::Win32::System::WindowsProgramming::INFINITE;
use librashader_common::{ImageFormat, Size};
use librashader_reflect::back::{CompileReflectShader, CompileShader};
use librashader_reflect::reflect::ReflectShader;
use librashader_reflect::reflect::semantics::{MAX_BINDINGS_COUNT, ShaderSemantics, TextureSemantics, UniformBinding};
use librashader_runtime::uniforms::UniformStorage;
use crate::buffer::{D3D12Buffer, D3D12ConstantBuffer};
use crate::filter_pass::FilterPass;
use crate::framebuffer::OwnedImage;
use crate::graphics_pipeline::{D3D12GraphicsPipeline, D3D12RootSignature};
use crate::mipmap::D3D12MipmapGen;
use crate::quad_render::DrawQuad;
use crate::texture::InputTexture;

type ShaderPassMeta = ShaderPassArtifact<impl CompileReflectShader<HLSL, GlslangCompilation>>;

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
        options: Option<&()>,
    ) -> error::Result<FilterChainD3D12> {
        // load passes from preset
        let preset = ShaderPreset::try_parse(path)?;
        Self::load_from_preset(device, preset, options)
    }

    /// Load a filter chain from a pre-parsed `ShaderPreset`.
    pub fn load_from_preset(
        device: &ID3D12Device,
        preset: ShaderPreset,
        _options: Option<&()>,
    ) -> error::Result<FilterChainD3D12> {
        let shader_count = preset.shaders.len();
        let lut_count = preset.textures.len();
        let (passes, semantics) = HLSL::compile_preset_passes::<GlslangCompilation, Box<dyn Error>>(
            preset.shaders,
            &preset.textures,
        )?;

        let samplers = SamplerSet::new(device)?;
        let mipmap_gen = D3D12MipmapGen::new(device).unwrap();

        let draw_quad = DrawQuad::new(device)?;
        let mut staging_heap =
            D3D12DescriptorHeap::new(device,
                                                         (MAX_BINDINGS_COUNT as usize) *
                                                             shader_count + 2048 + lut_count)?;



        let luts = FilterChainD3D12::load_luts(device, &mut staging_heap, &preset.textures, &mipmap_gen).unwrap();

        let root_signature = D3D12RootSignature::new(device)?;

        let filters = FilterChainD3D12::init_passes(device, &root_signature, passes, &semantics)?;




        // initialize output framebuffers
        let mut output_framebuffers = Vec::new();
        output_framebuffers.resize_with(filters.len(), || {
            OwnedImage::new(
                device,
                Size::new(1, 1),
                ImageFormat::R8G8B8A8Unorm,
                false,
            )
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
            OwnedImage::new(
                device,
                Size::new(1, 1),
                ImageFormat::R8G8B8A8Unorm,
                false,
            )
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
            passes: filters,
            output_framebuffers: output_framebuffers.into_boxed_slice(),
            feedback_framebuffers: feedback_framebuffers.into_boxed_slice(),
            history_framebuffers,
        })
    }

    fn init_history(
        device: &ID3D12Device,
        filters: &Vec<FilterPass>,
    ) -> error::Result<(VecDeque<OwnedImage>, Box<[Option<InputTexture>]>)> {
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
            OwnedImage::new(
                device,
                Size::new(1, 1),
                ImageFormat::R8G8B8A8Unorm,
                false,
            )
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
        mipmap_gen: &D3D12MipmapGen
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
        let mut work_heap: D3D12DescriptorHeap<ResourceWorkHeap> = D3D12DescriptorHeap::new(device, u16::MAX as usize)?;
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

            let residuals = mipmap_gen
                .mipmapping_context(&cmd, &mut work_heap, |context| {
                for lut in luts.values() {
                    lut.generate_mipmaps(context).unwrap()
                }
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

    fn init_passes(device: &ID3D12Device,
                   root_signature: &D3D12RootSignature,
                   passes: Vec<ShaderPassMeta>,
                   semantics: &ShaderSemantics,)
        -> error::Result<Vec<FilterPass>> {

        let mut filters = Vec::new();
        let shader_count = passes.len();
        let work_heap =
            D3D12DescriptorHeap::<ResourceWorkHeap>::new(device,
                                                         (MAX_BINDINGS_COUNT as usize) *
                                                             shader_count)?;
        let work_heaps = unsafe {
            work_heap.suballocate(shader_count)
        };


        let sampler_work_heap =
            D3D12DescriptorHeap::new(device,
                                     (MAX_BINDINGS_COUNT as usize) * shader_count)?;

        let sampler_work_heaps = unsafe {
            sampler_work_heap.suballocate(shader_count)
        };

        for (index, (((config, source, mut reflect),
            texture_heap), sampler_heap))
            in passes.into_iter()
            .zip(work_heaps)
            .zip(sampler_work_heaps)
            .enumerate() {
            let reflection = reflect.reflect(index, semantics)?;
            let hlsl = reflect.compile(None)?;

            let graphics_pipeline = D3D12GraphicsPipeline::new(device,
                                                               &hlsl,
                root_signature,
                if let Some(format) = config.get_format_override() {
                    format
                } else if source.format != ImageFormat::Unknown {
                    source.format
                } else {
                    ImageFormat::R8G8B8A8Unorm
                }.into()
            )?;

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

        Ok(filters)
    }

}
