use crate::draw_quad::DrawQuad;
use crate::error;
use crate::error::FilterChainError;
use crate::filter_pass::FilterPass;
use crate::luts::LutTexture;
use crate::options::FilterChainOptionsMetal;
use crate::samplers::SamplerSet;
use crate::texture::{MetalTexture, OwnedImage};
use icrate::Metal::{MTLCommandBuffer, MTLCommandQueue, MTLDevice};
use librashader_presets::context::VideoDriver;
use librashader_presets::{ShaderPassConfig, ShaderPreset, TextureConfig};
use librashader_reflect::back::targets::{MSL, WGSL};
use librashader_reflect::back::CompileReflectShader;
use librashader_reflect::front::{Glslang, SpirvCompilation};
use librashader_reflect::reflect::cross::SpirvCross;
use librashader_reflect::reflect::naga::Naga;
use librashader_reflect::reflect::presets::{CompilePresetTarget, ShaderPassArtifact};
use librashader_reflect::reflect::semantics::ShaderSemantics;
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

type ShaderPassMeta =
    ShaderPassArtifact<impl CompileReflectShader<MSL, SpirvCompilation, SpirvCross> + Send>;
fn compile_passes(
    shaders: Vec<ShaderPassConfig>,
    textures: &[TextureConfig],
) -> Result<(Vec<ShaderPassMeta>, ShaderSemantics), FilterChainError> {
    let (passes, semantics) =
        MSL::compile_preset_passes::<Glslang, SpirvCompilation, SpirvCross, FilterChainError>(
            shaders, &textures,
        )?;
    Ok((passes, semantics))
}

/// A wgpu filter chain.
pub struct FilterChainMetal {
    pub(crate) common: FilterCommon,
    passes: Box<[FilterPass]>,
    output_framebuffers: Box<[OwnedImage]>,
    feedback_framebuffers: Box<[OwnedImage]>,
    history_framebuffers: VecDeque<OwnedImage>,
    disable_mipmaps: bool,
}

pub struct FilterMutable {
    pub passes_enabled: usize,
    pub(crate) parameters: FxHashMap<String, f32>,
}

pub(crate) struct FilterCommon {
    pub output_textures: Box<[Option<MetalTexture>]>,
    pub feedback_textures: Box<[Option<MetalTexture>]>,
    pub history_textures: Box<[Option<MetalTexture>]>,
    pub luts: FxHashMap<usize, LutTexture>,
    pub samplers: SamplerSet,
    pub config: FilterMutable,
    pub internal_frame_count: i32,
    pub(crate) draw_quad: DrawQuad,
    device: Id<ProtocolObject<dyn MTLDevice>>,
    queue: Id<ProtocolObject<dyn MTLCommandQueue>>,
}

impl FilterChainMetal {
    /// Load the shader preset at the given path into a filter chain.
    pub fn load_from_path(
        path: impl AsRef<Path>,
        queue: Id<ProtocolObject<dyn MTLCommandQueue>>,
        options: Option<&FilterChainOptionsMetal>,
    ) -> error::Result<FilterChainMetal> {
        // load passes from preset
        let preset = ShaderPreset::try_parse_with_driver_context(path, VideoDriver::Metal)?;
        Self::load_from_preset(preset, queue, options)
    }

    /// Load a filter chain from a pre-parsed `ShaderPreset`.
    pub fn load_from_preset(
        preset: ShaderPreset,
        queue: Id<ProtocolObject<dyn MTLCommandQueue>>,
        options: Option<&FilterChainOptionsMetal>,
    ) -> error::Result<FilterChainMetal> {
        let cmd = queue
            .commandBuffer()
            .ok_or(FilterChainError::FailedToCreateCommandBuffer)?;

        let filter_chain = Self::load_from_preset_deferred(preset, queue, cmd, options)?;

        cmd.commit();
        unsafe { cmd.waitUntilCompleted() };

        Ok(filter_chain)
    }

    /// Load a filter chain from a pre-parsed `ShaderPreset`, deferring and GPU-side initialization
    /// to the caller. This function therefore requires no external synchronization of the device queue.
    ///
    /// ## Safety
    /// The provided command buffer must be ready for recording.
    /// The caller is responsible for ending the command buffer and immediately submitting it to a
    /// graphics queue. The command buffer must be completely executed before calling [`frame`](Self::frame).
    pub fn load_from_preset_deferred(
        preset: ShaderPreset,
        queue: Id<ProtocolObject<dyn MTLCommandQueue>>,
        cmd: Id<ProtocolObject<dyn MTLCommandBuffer>>,
        options: Option<&FilterChainOptionsMetal>,
    ) -> error::Result<FilterChainMetal> {
        let device = queue.device();
        let (passes, semantics) = compile_passes(preset.shaders, &preset.textures)?;

        let samplers = SamplerSet::new(&device)?;
    }
}
