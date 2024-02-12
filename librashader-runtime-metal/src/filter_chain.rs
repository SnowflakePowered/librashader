use crate::draw_quad::DrawQuad;
use crate::error;
use crate::error::FilterChainError;
use crate::filter_pass::FilterPass;
use crate::luts::LutTexture;
use crate::options::FilterChainOptionsMetal;
use crate::samplers::SamplerSet;
use crate::texture::{MetalTexture, OwnedImage};
use icrate::Metal::{MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDevice, MTLPixelFormat, MTLPixelFormatRGBA8Unorm, MTLTexture};
use librashader_presets::context::VideoDriver;
use librashader_presets::{ShaderPassConfig, ShaderPreset, TextureConfig};
use librashader_reflect::back::targets::{MSL, WGSL};
use librashader_reflect::back::CompileReflectShader;
use librashader_reflect::front::{Glslang, SpirvCompilation};
use librashader_reflect::reflect::cross::SpirvCross;
use librashader_reflect::reflect::naga::{Naga, NagaLoweringOptions};
use librashader_reflect::reflect::presets::{CompilePresetTarget, ShaderPassArtifact};
use librashader_reflect::reflect::semantics::ShaderSemantics;
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use librashader_common::{ImageFormat, Size};
use librashader_reflect::back::msl::MslVersion;
use librashader_runtime::framebuffer::FramebufferInit;
use librashader_runtime::image::{Image, ImageError, UVDirection};
use librashader_runtime::uniforms::UniformStorage;
use crate::buffer::MetalBuffer;
use crate::graphics_pipeline::MetalGraphicsPipeline;

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

    fn load_luts(
        device: &ProtocolObject<dyn MTLDevice>,
        cmd: &ProtocolObject<dyn MTLCommandBuffer>,
        textures: &[TextureConfig],
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
        let mut luts = FxHashMap::default();

        let mipmapper = cmd.blitCommandEncoder()
            .ok_or(FilterChainError::FailedToCreateCommandBuffer)?;

        let images = textures.par_iter()
            .map(|texture| Image::load(&texture.path, UVDirection::TopLeft))
            .collect::<Result<Vec<Image>, ImageError>>()?;
        for (index, (texture, image)) in textures.iter().zip(images).enumerate() {
            let texture =
                LutTexture::new(device, &mipmapper, image, texture)?;
            luts.insert(index, texture);
        }

        mipmapper.endEncoding();
        Ok(luts)
    }

    fn init_passes(
        device: &Id<ProtocolObject<dyn MTLDevice>>,
        passes: Vec<ShaderPassMeta>,
        semantics: &ShaderSemantics,
    ) -> error::Result<Box<[FilterPass]>> {
        let filters: Vec<error::Result<FilterPass>> = passes.into_par_iter()
            .enumerate()
            .map(|(index, (config, source, mut reflect))| {
                let reflection = reflect.reflect(index, semantics)?;
                let msl = reflect.compile(MslVersion::V2_0)?;

                let ubo_size = reflection.ubo.as_ref().map_or(0, |ubo| ubo.size as usize);
                let push_size = reflection
                    .push_constant
                    .as_ref()
                    .map_or(0, |push| push.size);

                let uniform_storage = UniformStorage::new_with_storage(
                    MetalBuffer::new(&device, ubo_size)?,
                    MetalBuffer::new(&device, push_size)?,
                );

                let uniform_bindings = reflection.meta.create_binding_map(|param| param.offset());

                let render_pass_format: MTLPixelFormat =
                    if let Some(format) = config.get_format_override() {
                        format.into()
                    } else {
                        source.format.into()
                    };

                let graphics_pipeline = MetalGraphicsPipeline::new(
                    Id::clone(&device),
                    &msl,
                    if render_pass_format == 0 {
                        MTLPixelFormatRGBA8Unorm
                    } else {
                        render_pass_format
                    }
                )?;

                Ok(FilterPass {
                    device: Id::clone(&device),
                    reflection,
                    uniform_storage,
                    uniform_bindings,
                    source,
                    config,
                    graphics_pipeline,
                })
            })
            .collect();
        //
        let filters: error::Result<Vec<FilterPass>> = filters.into_iter().collect();
        let filters = filters?;
        Ok(filters.into_boxed_slice())
    }

    fn push_history(&mut self, input: &ProtocolObject<dyn MTLTexture>,
                    cmd: &ProtocolObject<dyn MTLBlitCommandEncoder>) {
        if let Some(mut back) = self.history_framebuffers.pop_back() {
            if back.image.height() != input.height()
                || back.image.width() != input.width()
                || input.pixelFormat() != back.image.pixelFormat() {

                let size = Size {
                    width: input.width() as u32,
                    height: input.height() as u32
                };

                let _old_back = std::mem::replace(
                    &mut back,
                    OwnedImage::new(
                        &self.common.device,
                        size,
                        1,
                        input.pixelFormat(),
                    )?,
                );
            }

            back.copy_from(cmd, input)?;

            self.history_framebuffers.push_front(back)
        }
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

        let filters = Self::init_passes(&device, passes, &semantics)?;

        let samplers = SamplerSet::new(&device)?;
        let luts = FilterChainMetal::load_luts(
            &device,
            &cmd,
            &preset.textures,
        )?;
        let framebuffer_gen = || {
            Ok::<_, error::FilterChainError>(OwnedImage::new(
                &device,
                Size::new(1, 1),
                1,
                ImageFormat::R8G8B8A8Unorm.into(),
            ))
        };
        let input_gen = || None;
        let framebuffer_init = FramebufferInit::new(
            filters.iter().map(|f| &f.reflection.meta),
            &framebuffer_gen,
            &input_gen,
        );
        let (output_framebuffers, output_textures) = framebuffer_init.init_output_framebuffers()?;
        //
        // initialize feedback framebuffers
        let (feedback_framebuffers, feedback_textures) =
            framebuffer_init.init_output_framebuffers()?;
        //
        // initialize history
        let (history_framebuffers, history_textures) = framebuffer_init.init_history()?;

        let draw_quad = DrawQuad::new(&device)?;
        Ok(FilterChainMetal{
            common: FilterCommon {
                luts,
                samplers,
                config: FilterMutable {
                    passes_enabled: preset.shader_count as usize,
                    parameters: preset
                        .parameters
                        .into_iter()
                        .map(|param| (param.name, param.value))
                        .collect(),
                },
                draw_quad,
                device,
                queue,
                output_textures,
                feedback_textures,
                history_textures,
                internal_frame_count: 0,
            },
            passes: filters,
            output_framebuffers,
            feedback_framebuffers,
            history_framebuffers,
            disable_mipmaps: options.map(|f| f.force_no_mipmaps).unwrap_or(false),
        })
    }


}
