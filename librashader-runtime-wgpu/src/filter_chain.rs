use librashader_presets::{ShaderPassConfig, ShaderPreset, TextureConfig};
use librashader_reflect::back::targets::{SPIRV, WGSL};
use librashader_reflect::back::{CompileReflectShader, CompileShader};
use librashader_reflect::front::GlslangCompilation;
use librashader_reflect::reflect::presets::{CompilePresetTarget, ShaderPassArtifact};
use librashader_reflect::reflect::semantics::ShaderSemantics;
use librashader_reflect::reflect::ReflectShader;
use librashader_runtime::binding::BindingUtil;
use librashader_runtime::image::{Image, ImageError, UVDirection, BGRA8, RGBA8};
use librashader_runtime::quad::QuadType;
use librashader_runtime::uniforms::UniformStorage;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::path::Path;
use std::sync::Arc;

use librashader_common::{ImageFormat, Size, Viewport};
use librashader_reflect::back::wgsl::WgslCompileOptions;
use librashader_runtime::framebuffer::FramebufferInit;
use librashader_runtime::render_target::RenderTarget;
use librashader_runtime::scaling::ScaleFramebuffer;
use rayon::prelude::*;
use wgpu::{BindGroupEntry, CommandBuffer, CommandEncoder, Device, Queue, TextureAspect, TextureFormat};
use crate::buffer::WgpuMappedBuffer;
use crate::draw_quad::DrawQuad;

use crate::error;
use crate::error::FilterChainError;
use crate::filter_pass::FilterPass;
use crate::framebuffer::OutputImage;
use crate::graphics_pipeline::WgpuGraphicsPipeline;
use crate::luts::LutTexture;
use crate::options::FrameOptionsWGPU;
use crate::samplers::SamplerSet;
use crate::texture::{Handle, InputImage, OwnedImage};

type ShaderPassMeta =
    ShaderPassArtifact<impl CompileReflectShader<WGSL, GlslangCompilation> + Send>;
fn compile_passes(
    shaders: Vec<ShaderPassConfig>,
    textures: &[TextureConfig],
) -> Result<(Vec<ShaderPassMeta>, ShaderSemantics), FilterChainError> {
    let (passes, semantics) =
        WGSL::compile_preset_passes::<GlslangCompilation, FilterChainError>(shaders, &textures)?;
    Ok((passes, semantics))
}

/// A Vulkan filter chain.
pub struct FilterChainWGPU {
    pub(crate) common: FilterCommon,
    passes: Box<[FilterPass]>,
    output_framebuffers: Box<[OwnedImage]>,
    feedback_framebuffers: Box<[OwnedImage]>,
    history_framebuffers: VecDeque<OwnedImage>,
    disable_mipmaps: bool,
    // residuals: Box<[FrameResiduals]>,
}

pub struct FilterMutable {
    pub passes_enabled: usize,
    pub(crate) parameters: FxHashMap<String, f32>,
}

pub(crate) struct FilterCommon {
    pub output_textures: Box<[Option<InputImage>]>,
    pub feedback_textures: Box<[Option<InputImage>]>,
    pub history_textures: Box<[Option<InputImage>]>,
    pub luts: FxHashMap<usize, LutTexture>,
    pub samplers: SamplerSet,
    pub config: FilterMutable,
    pub internal_frame_count: i32,
    pub(crate) draw_quad: DrawQuad,
    device: Arc<Device>,
}

impl FilterChainWGPU {
    /// Load a filter chain from a pre-parsed `ShaderPreset`, deferring and GPU-side initialization
    /// to the caller. This function therefore requires no external synchronization of the device queue.
    ///
    /// ## Safety
    /// The provided command buffer must be ready for recording and contain no prior commands.
    /// The caller is responsible for ending the command buffer and immediately submitting it to a
    /// graphics queue. The command buffer must be completely executed before calling [`frame`](Self::frame).
    pub fn load_from_preset_deferred(
        device: Arc<Device>,
        queue: &mut wgpu::Queue,
        cmd: &mut wgpu::CommandEncoder,
        preset: ShaderPreset,
    ) -> error::Result<FilterChainWGPU> {
        let (passes, semantics) = compile_passes(preset.shaders, &preset.textures)?;

        // // initialize passes
        let filters = Self::init_passes(Arc::clone(&device), passes, &semantics)?;
        //
        let luts = FilterChainWGPU::load_luts(&device, queue, cmd, &preset.textures)?;
        let samplers = SamplerSet::new(&device);
        //
        let framebuffer_gen =
            || Ok::<_, error::FilterChainError>(OwnedImage::new(Arc::clone(&device), Size::new(1, 1), 1, ImageFormat::R8G8B8A8Unorm));
        let input_gen = || None;
        let framebuffer_init = FramebufferInit::new(
            filters.iter().map(|f| &f.reflection.meta),
            &framebuffer_gen,
            &input_gen,
        );

        //
        // // initialize output framebuffers
        let (output_framebuffers, output_textures) = framebuffer_init.init_output_framebuffers()?;
        //
        // initialize feedback framebuffers
        let (feedback_framebuffers, feedback_textures) =
            framebuffer_init.init_output_framebuffers()?;
        //
        // initialize history
        let (history_framebuffers, history_textures) = framebuffer_init.init_history()?;
        //
        // let mut intermediates = Vec::new();
        // intermediates.resize_with(frames_in_flight as usize, || {
        //     FrameResiduals::new(&device.device)
        // });

        let draw_quad = DrawQuad::new(&device, queue);

        Ok(FilterChainWGPU {
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
                        output_textures,
                        feedback_textures,
                        history_textures,
                        internal_frame_count: 0,

            },
            passes: filters,
                output_framebuffers,
                feedback_framebuffers,
                history_framebuffers,
                disable_mipmaps: false // todo: force no mipmaps,
        })
    }

    fn load_luts(
        device: &wgpu::Device,
        queue: &mut wgpu::Queue,
        cmd: &mut wgpu::CommandEncoder,
        textures: &[TextureConfig],
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
        let mut luts = FxHashMap::default();
        let images = textures
            .par_iter()
            .map(|texture| Image::load(&texture.path, UVDirection::TopLeft))
            .collect::<Result<Vec<Image>, ImageError>>()?;
        for (index, (texture, image)) in textures.iter().zip(images).enumerate() {
            let texture = LutTexture::new(device, queue, cmd, image, texture);
            luts.insert(index, texture);
        }
        Ok(luts)
    }

    fn init_passes(
        device: Arc<Device>,
        passes: Vec<ShaderPassMeta>,
        semantics: &ShaderSemantics,
    ) -> error::Result<Box<[FilterPass]>> {
        // let frames_in_flight = std::cmp::max(1, frames_in_flight);
        //
        let filters: Vec<error::Result<FilterPass>> = passes
            .into_par_iter()
            .enumerate()
            .map(|(index, (config, source, mut reflect))| {
                let reflection = reflect.reflect(index, semantics)?;
                let wgsl = reflect.compile(WgslCompileOptions {
                    write_pcb_as_ubo: true,
                    sampler_bind_group: 1,
                })?;

                let ubo_size = reflection.ubo.as_ref().map_or(0, |ubo| ubo.size as usize);
                let push_size = reflection
                    .push_constant
                    .as_ref()
                    .map_or(0, |push| push.size as wgpu::BufferAddress);


                let uniform_storage = UniformStorage::new_with_storage(
                    WgpuMappedBuffer::new(&device, wgpu::BufferUsages::UNIFORM, ubo_size as wgpu::BufferAddress, Some("ubo")),
                    WgpuMappedBuffer::new(&device, wgpu::BufferUsages::UNIFORM, push_size as wgpu::BufferAddress, Some("push"))
                );

                let uniform_bindings = reflection.meta.create_binding_map(|param| param.offset());

                let render_pass_format: Option<TextureFormat> =
                    if let Some(format) = config.get_format_override() {
                        format.into()
                    } else {
                        source.format.into()
                    };

                let graphics_pipeline = WgpuGraphicsPipeline::new(
                    Arc::clone(&device),
                    &wgsl,
                    &reflection,
                    render_pass_format.unwrap_or(TextureFormat::R8Unorm),
                );



                Ok(FilterPass {
                    device: Arc::clone(&device),
                    reflection,
                    compiled: wgsl,
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

    pub fn frame<'a>(&mut self,
        input: wgpu::Texture,
        viewport: &Viewport<OutputImage<'a>>,
        cmd: &mut wgpu::CommandEncoder,
        frame_count: usize,
                 options: Option<&FrameOptionsWGPU>,

    ) -> error::Result<()> {
        let max = std::cmp::min(self.passes.len(), self.common.config.passes_enabled);
        let passes = &mut self.passes[0..max];

        if let Some(options) = &options {
            if options.clear_history {
                for history in &mut self.history_framebuffers {
                    // history.clear(cmd);
                }
            }
        }

        if passes.is_empty() {
            return Ok(());
        }

        let original_image_view = input.create_view(&wgpu::TextureViewDescriptor {
            label: Some("original_image_view"),
            format: Some(input.format()),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 1,
            mip_level_count: None,
            base_array_layer: 1,
            array_layer_count: None,
        });

        let filter = passes[0].config.filter;
        let wrap_mode = passes[0].config.wrap_mode;


        // update history
        for (texture, image) in self
            .common
            .history_textures
            .iter_mut()
            .zip(self.history_framebuffers.iter())
        {
            *texture = Some(image.as_input(filter, wrap_mode));
        }

        let original = InputImage {
            image: Arc::new(input),
            view: Arc::new(original_image_view),
            wrap_mode,
            filter_mode: filter,
            mip_filter: filter,
        };

        let mut source = original.clone();

        // swap output and feedback **before** recording command buffers
        std::mem::swap(
            &mut self.output_framebuffers,
            &mut self.feedback_framebuffers,
        );

        // rescale render buffers to ensure all bindings are valid.
        OwnedImage::scale_framebuffers_with_context(
            source.image.size().into(),
            viewport.output.size,
            &mut self.output_framebuffers,
            &mut self.feedback_framebuffers,
            passes,
            &(),
            Some(&mut |index: usize,
                       pass: &FilterPass,
                       output: &OwnedImage,
                       feedback: &OwnedImage| {
                // refresh inputs
                self.common.feedback_textures[index] =
                    Some(feedback.as_input(pass.config.filter, pass.config.wrap_mode));
                self.common.output_textures[index] =
                    Some(output.as_input(pass.config.filter, pass.config.wrap_mode));
                Ok(())
            }),
        )?;

        let passes_len = passes.len();
        let (pass, last) = passes.split_at_mut(passes_len - 1);

        let frame_direction = options.map_or(1, |f| f.frame_direction);


        for (index, pass) in pass.iter_mut().enumerate() {
            let target = &self.output_framebuffers[index];
            source.filter_mode = pass.config.filter;
            source.wrap_mode = pass.config.wrap_mode;
            source.mip_filter = pass.config.filter;

            let output_image = OutputImage::new(target);
            let out = RenderTarget::identity(&output_image);

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
                QuadType::Offscreen
            )?;

            if target.max_miplevels > 1 && !self.disable_mipmaps {
                // todo: mipmaps
            }

            source = self.common.output_textures[index].clone().unwrap();
        }

        // try to hint the optimizer
        assert_eq!(last.len(), 1);

        if let Some(pass) = last.iter_mut().next() {
            if pass.graphics_pipeline.format != viewport.output.format {
                // need to recompile
                pass.graphics_pipeline.recompile(viewport.output.format);
            }
            source.filter_mode = pass.config.filter;
            source.wrap_mode = pass.config.wrap_mode;
            source.mip_filter = pass.config.filter;
            let output_image = &viewport.output;
            let out = RenderTarget::viewport_with_output(output_image, viewport);
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
                QuadType::Final
            )?;
        }

        // self.push_history(input, cmd)?;
        self.common.internal_frame_count = self.common.internal_frame_count.wrapping_add(1);
        Ok(())
    }

}
