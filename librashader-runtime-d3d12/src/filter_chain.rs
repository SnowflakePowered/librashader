use std::borrow::Borrow;
use crate::{error, util};
use crate::heap::{D3D12DescriptorHeap, LutTextureHeap, ResourceWorkHeap};
use crate::samplers::SamplerSet;
use crate::texture::LutTexture;
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
    ID3D12CommandAllocator, ID3D12CommandList, ID3D12CommandQueue, ID3D12Device, ID3D12Fence,
    ID3D12GraphicsCommandList, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
    D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_FENCE_FLAG_NONE,
};
use windows::Win32::System::Threading::{CreateEventA, ResetEvent, WaitForSingleObject};
use windows::Win32::System::WindowsProgramming::INFINITE;
use librashader_common::ImageFormat;
use librashader_reflect::back::{CompileReflectShader, CompileShader};
use librashader_reflect::reflect::ReflectShader;
use librashader_reflect::reflect::semantics::ShaderSemantics;
use crate::filter_pass::FilterPass;
use crate::graphics_pipeline::{D3D12GraphicsPipeline, D3D12RootSignature};
use crate::mipmap::D3D12MipmapGen;
use crate::quad_render::DrawQuad;

type ShaderPassMeta = ShaderPassArtifact<impl CompileReflectShader<HLSL, GlslangCompilation>>;

pub struct FilterChainD3D12 {
    pub(crate) common: FilterCommon,
    // pub(crate) passes: Vec<FilterPass>,
    // pub(crate) output_framebuffers: Box<[OwnedFramebuffer]>,
    // pub(crate) feedback_framebuffers: Box<[OwnedFramebuffer]>,
    // pub(crate) history_framebuffers: VecDeque<OwnedFramebuffer>,
    // pub(crate) draw_quad: DrawQuad,
    pub(crate) filters: Vec<FilterPass>,
}

pub(crate) struct FilterCommon {
    pub(crate) d3d12: ID3D12Device,
    // pub(crate) luts: FxHashMap<usize, LutTexture>,
    pub samplers: SamplerSet,
    // pub output_textures: Box<[Option<InputTexture>]>,
    // pub feedback_textures: Box<[Option<InputTexture>]>,
    // pub history_textures: Box<[Option<InputTexture>]>,
    // pub config: FilterMutable,
    // pub disable_mipmaps: bool,
    lut_heap: D3D12DescriptorHeap<LutTextureHeap>,
    luts: FxHashMap<usize, LutTexture>,
    mipmap_gen: D3D12MipmapGen,
    root_signature: D3D12RootSignature,
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
        options: Option<&()>,
    ) -> error::Result<FilterChainD3D12> {
        let (passes, semantics) = HLSL::compile_preset_passes::<GlslangCompilation, Box<dyn Error>>(
            preset.shaders,
            &preset.textures,
        )?;

        let samplers = SamplerSet::new(device)?;
        let mipmap_gen = D3D12MipmapGen::new(device).unwrap();

        let mut lut_heap = D3D12DescriptorHeap::new(device, preset.textures.len())?;
        let luts = FilterChainD3D12::load_luts(device, &mut lut_heap, &preset.textures, &mipmap_gen).unwrap();

        let root_signature = D3D12RootSignature::new(device)?;

        let filters = FilterChainD3D12::init_passes(device, &root_signature, passes, &semantics)?;

        Ok(FilterChainD3D12 {
            common: FilterCommon {
                d3d12: device.clone(),
                samplers,
                lut_heap,
                luts,
                mipmap_gen,
                root_signature,
            },
            filters
        })
    }

    fn load_luts(
        device: &ID3D12Device,
        heap: &mut D3D12DescriptorHeap<LutTextureHeap>,
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
        for (index, (config, source, mut reflect)) in passes.into_iter().enumerate() {
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

            filters.push(FilterPass {
                pipeline: graphics_pipeline,
            })

        }

        Ok(filters)
    }

}
