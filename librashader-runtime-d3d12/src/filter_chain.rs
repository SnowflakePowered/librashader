use std::error::Error;
use std::path::Path;
use rustc_hash::FxHashMap;
use windows::Win32::Graphics::Direct3D12::ID3D12Device;
use librashader_presets::{ShaderPreset, TextureConfig};
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::front::GlslangCompilation;
use librashader_reflect::reflect::presets::CompilePresetTarget;
use crate::error;
use crate::samplers::SamplerSet;
use crate::texture::LutTexture;

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
        textures: &[TextureConfig],
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
        todo!()
    }

    /// Load a filter chain from a pre-parsed `ShaderPreset`.
    pub fn load_from_preset(
        device: &ID3D12Device,
        preset: ShaderPreset,
        options: Option<&()>,
    ) -> error::Result<FilterChainD3D12> {
        let (passes, semantics) = HLSL::compile_preset_passes::<
            GlslangCompilation,
            Box<dyn Error>,
        >(preset.shaders, &preset.textures)?;

        let samplers = SamplerSet::new(&device)?;
        Ok(FilterChainD3D12 {
            common: FilterCommon {
                d3d12: device.clone(),
                samplers,
            },
        })
    }
}