use crate::error;
use crate::heap::{D3D12DescriptorHeap, LutTextureHeap};
use crate::samplers::SamplerSet;
use crate::texture::LutTexture;
use librashader_presets::{ShaderPreset, TextureConfig};
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::front::GlslangCompilation;
use librashader_reflect::reflect::presets::CompilePresetTarget;
use librashader_runtime::image::{Image, UVDirection};
use rustc_hash::FxHashMap;
use std::error::Error;
use std::path::Path;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D12::{
    ID3D12CommandAllocator, ID3D12CommandList, ID3D12CommandQueue, ID3D12Device, ID3D12Fence,
    ID3D12GraphicsCommandList, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
    D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_FENCE_FLAG_NONE,
};
use windows::Win32::System::Threading::{CreateEventA, WaitForSingleObject};
use windows::Win32::System::WindowsProgramming::INFINITE;

pub struct FilterChainD3D12 {
    pub(crate) common: FilterCommon,
    // pub(crate) passes: Vec<FilterPass>,
    // pub(crate) output_framebuffers: Box<[OwnedFramebuffer]>,
    // pub(crate) feedback_framebuffers: Box<[OwnedFramebuffer]>,
    // pub(crate) history_framebuffers: VecDeque<OwnedFramebuffer>,
    // pub(crate) draw_quad: DrawQuad,
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

    fn load_luts(
        device: &ID3D12Device,
        heap: &mut D3D12DescriptorHeap<LutTextureHeap>,
        textures: &[TextureConfig],
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
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

            let mut luts = FxHashMap::default();

            for (index, texture) in textures.iter().enumerate() {
                let image = Image::load(&texture.path, UVDirection::TopLeft)?;

                let texture = LutTexture::new(
                    device,
                    heap,
                    &cmd,
                    &image,
                    texture.filter_mode,
                    texture.wrap_mode,
                    // todo: mipmaps
                    false,
                )?;
                luts.insert(index, texture);
            }

            cmd.Close()?;

            queue.ExecuteCommandLists(&[cmd.cast()?]);
            queue.Signal(&fence, 1)?;

            // Wait until the previous frame is finished.
            if unsafe { fence.GetCompletedValue() } < 1 {
                unsafe { fence.SetEventOnCompletion(1, fence_event) }
                    .ok()
                    .unwrap();

                unsafe { WaitForSingleObject(fence_event, INFINITE) };
            }
            Ok(luts)
        }
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
        let mut lut_heap = D3D12DescriptorHeap::new(device, preset.textures.len())?;

        let luts = FilterChainD3D12::load_luts(device, &mut lut_heap, &preset.textures)?;

        Ok(FilterChainD3D12 {
            common: FilterCommon {
                d3d12: device.clone(),
                samplers,
                lut_heap,
                luts,
            },
        })
    }
}
