use crate::draw_quad::DrawQuad;
use crate::error::FilterChainError;
use crate::filter_pass::FilterPass;
use crate::framebuffer::OutputImage;
use crate::luts::LutTexture;
use crate::queue_selection::get_graphics_queue;
use crate::render_target::{RenderTarget, DEFAULT_MVP};
use crate::samplers::SamplerSet;
use crate::texture::{InputImage, OwnedImage, VulkanImage};
use crate::ubo_ring::VkUboRing;
use crate::vulkan_state::VulkanGraphicsPipeline;
use crate::{error, util};
use ash::vk;
use librashader_common::{ImageFormat, Size, Viewport};
use librashader_preprocess::ShaderSource;
use librashader_presets::{ShaderPassConfig, ShaderPreset, TextureConfig};
use librashader_reflect::back::targets::SpirV;
use librashader_reflect::back::{CompileShader, CompilerBackend, FromCompilation};
use librashader_reflect::front::shaderc::GlslangCompilation;
use librashader_reflect::reflect::semantics::{
    Semantic, ShaderSemantics, TextureSemantics, UniformBinding, UniformSemantic, UniqueSemantics,
};
use librashader_reflect::reflect::ReflectShader;
use librashader_runtime::image::{Image, UVDirection};
use librashader_runtime::uniforms::UniformStorage;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::Path;
use crate::options::{FilterChainOptions, FrameOptions};

/// A Vulkan device and metadata that is required by the shader runtime.
pub struct VulkanDevice {
    pub(crate) device: ash::Device,
    pub(crate) memory_properties: vk::PhysicalDeviceMemoryProperties,
    queue: vk::Queue,
    pipeline_cache: vk::PipelineCache,
}

type ShaderPassMeta = (
    ShaderPassConfig,
    ShaderSource,
    CompilerBackend<impl CompileShader<SpirV, Options = Option<()>, Context = ()> + ReflectShader>,
);

/// A collection of handles needed to access the Vulkan instance.
#[derive(Clone)]
pub struct VulkanInstance {
    /// A `VkDevice` handle.
    pub device: vk::Device,
    /// A `VkInstance` handle.
    pub instance: vk::Instance,
    /// A `VkPhysicalDevice` handle.
    pub physical_device: vk::PhysicalDevice,
    /// A function pointer to the Vulkan library entry point.
    pub get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
}

impl TryFrom<VulkanInstance> for VulkanDevice {
    type Error = FilterChainError;

    fn try_from(vulkan: VulkanInstance) -> Result<Self, FilterChainError> {
        unsafe {
            let instance = ash::Instance::load(
                &vk::StaticFn {
                    get_instance_proc_addr: vulkan.get_instance_proc_addr,
                },
                vulkan.instance,
            );

            let device = ash::Device::load(instance.fp_v1_0(), vulkan.device);

            let pipeline_cache =
                device.create_pipeline_cache(&vk::PipelineCacheCreateInfo::default(), None)?;

            let queue = get_graphics_queue(&instance, &device, vulkan.physical_device);
            let memory_properties =
                instance.get_physical_device_memory_properties(vulkan.physical_device);

            Ok(VulkanDevice {
                device,
                queue,
                pipeline_cache,
                memory_properties,
                // debug,
            })
        }
    }
}

impl TryFrom<(vk::PhysicalDevice, ash::Instance, ash::Device)> for VulkanDevice {
    type Error = FilterChainError;

    fn try_from(value: (vk::PhysicalDevice, ash::Instance, ash::Device)) -> error::Result<Self> {
        unsafe {
            let device = value.2;

            let pipeline_cache = device.create_pipeline_cache(&vk::PipelineCacheCreateInfo::default(), None)?;

            let queue = get_graphics_queue(&value.1, &device, value.0);

            let memory_properties = value.1.get_physical_device_memory_properties(value.0);

            Ok(VulkanDevice {
                device,
                queue,
                pipeline_cache,
                memory_properties,
                // debug: value.3,
            })
        }
    }
}

/// A Vulkan filter chain.
pub struct FilterChain {
    pub(crate) common: FilterCommon,
    passes: Box<[FilterPass]>,
    vulkan: VulkanDevice,
    output_framebuffers: Box<[OwnedImage]>,
    feedback_framebuffers: Box<[OwnedImage]>,
    history_framebuffers: VecDeque<OwnedImage>,
    disable_mipmaps: bool,
    intermediates: Box<[FrameIntermediates]>,
}

pub struct FilterMutable {
    pub(crate) passes_enabled: usize,
    pub(crate) parameters: FxHashMap<String, f32>,
}

pub(crate) struct FilterCommon {
    pub(crate) luts: FxHashMap<usize, LutTexture>,
    pub samplers: SamplerSet,
    pub(crate) draw_quad: DrawQuad,
    pub output_inputs: Box<[Option<InputImage>]>,
    pub feedback_inputs: Box<[Option<InputImage>]>,
    pub history_textures: Box<[Option<InputImage>]>,
    pub config: FilterMutable,
    pub device: ash::Device,
}

/// Contains residual intermediate `VkImageView` and `VkImage` objects created
/// for intermediate shader passes.
///
/// These Vulkan objects must stay alive until the command buffer is submitted
/// to the rendering queue, and the GPU is done with the objects.
#[must_use]
pub struct FrameIntermediates {
    device: ash::Device,
    image_views: Vec<vk::ImageView>,
    owned: Vec<OwnedImage>,
}

impl FrameIntermediates {
    pub(crate) fn new(device: &ash::Device) -> Self {
        FrameIntermediates {
            device: device.clone(),
            image_views: Vec::new(),
            owned: Vec::new(),
        }
    }

    pub(crate) fn dispose_outputs(&mut self, output_framebuffer: OutputImage) {
        self.image_views.push(output_framebuffer.image_view);
    }

    pub(crate) fn dispose_owned(&mut self, owned: OwnedImage) {
        self.owned.push(owned)
    }

    /// Dispose of the intermediate objects created during a frame.
    pub fn dispose(&mut self) {
        for image_view in &self.image_views {
            if *image_view != vk::ImageView::null() {
                unsafe {
                    self.device.destroy_image_view(*image_view, None);
                }
            }
        }
        self.owned.clear()
    }
}

impl FilterChain {
    /// Load the shader preset at the given path into a filter chain.
    pub fn load_from_path(
        vulkan: impl TryInto<VulkanDevice, Error = FilterChainError>,
        path: impl AsRef<Path>,
        options: Option<&FilterChainOptions>,
    ) -> error::Result<FilterChain> {
        // load passes from preset
        let preset = ShaderPreset::try_parse(path)?;
        Self::load_from_preset(vulkan, preset, options)
    }

    /// Load a filter chain from a pre-parsed `ShaderPreset`.
    pub fn load_from_preset(
        vulkan: impl TryInto<VulkanDevice, Error = FilterChainError>,
        preset: ShaderPreset,
        options: Option<&FilterChainOptions>,
    ) -> error::Result<FilterChain> {
        let (passes, semantics) = FilterChain::load_preset(preset.shaders, &preset.textures)?;
        let device = vulkan.try_into()?;

        let mut frames_in_flight = options.map(|o| o.frames_in_flight).unwrap_or(0);
        if frames_in_flight == 0 {
            frames_in_flight = 3;
        }

        // initialize passes
        let filters = Self::init_passes(&device, passes, &semantics, frames_in_flight)?;

        let luts = FilterChain::load_luts(&device, &preset.textures)?;
        let samplers = SamplerSet::new(&device.device)?;

        let (history_framebuffers, history_textures) =
            FilterChain::init_history(&device, &filters)?;

        let mut output_framebuffers = Vec::new();
        output_framebuffers.resize_with(filters.len(), || {
            OwnedImage::new(&device, Size::new(1, 1), ImageFormat::R8G8B8A8Unorm, 1)
        });

        let mut feedback_framebuffers = Vec::new();
        feedback_framebuffers.resize_with(filters.len(), || {
            OwnedImage::new(&device, Size::new(1, 1), ImageFormat::R8G8B8A8Unorm, 1)
        });

        let output_framebuffers: error::Result<Vec<OwnedImage>> =
            output_framebuffers.into_iter().collect();
        let mut output_textures = Vec::new();
        output_textures.resize_with(filters.len(), || None);

        let feedback_framebuffers: error::Result<Vec<OwnedImage>> =
            feedback_framebuffers.into_iter().collect();
        let mut feedback_textures = Vec::new();
        feedback_textures.resize_with(filters.len(), || None);

        let mut intermediates = Vec::new();
        intermediates.resize_with(frames_in_flight as usize, || FrameIntermediates::new(&device.device));

        Ok(FilterChain {
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
                draw_quad: DrawQuad::new(&device.device, &device.memory_properties)?,
                device: device.device.clone(),
                output_inputs: output_textures.into_boxed_slice(),
                feedback_inputs: feedback_textures.into_boxed_slice(),
                history_textures,
            },
            passes: filters,
            vulkan: device,
            output_framebuffers: output_framebuffers?.into_boxed_slice(),
            feedback_framebuffers: feedback_framebuffers?.into_boxed_slice(),
            history_framebuffers,
            intermediates: intermediates.into_boxed_slice(),
            disable_mipmaps: options.map_or(false, |o| o.force_no_mipmaps),
        })
    }

    fn load_preset(
        passes: Vec<ShaderPassConfig>,
        textures: &[TextureConfig],
    ) -> error::Result<(Vec<ShaderPassMeta>, ShaderSemantics)> {
        let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();
        let mut texture_semantics: FxHashMap<String, Semantic<TextureSemantics>> =
            Default::default();

        let passes = passes
            .into_iter()
            .map(|shader| {
                // eprintln!("[vk] loading {}", &shader.name.display());
                let source: ShaderSource = ShaderSource::load(&shader.name)?;

                let spirv = GlslangCompilation::compile(&source)?;
                let reflect = SpirV::from_compilation(spirv)?;

                for parameter in source.parameters.iter() {
                    uniform_semantics.insert(
                        parameter.id.clone(),
                        UniformSemantic::Unique(Semantic {
                            semantics: UniqueSemantics::FloatParameter,
                            index: (),
                        }),
                    );
                }
                Ok::<_, FilterChainError>((shader, source, reflect))
            })
            .into_iter()
            .collect::<error::Result<Vec<(ShaderPassConfig, ShaderSource, CompilerBackend<_>)>>>(
            )?;

        for details in &passes {
            librashader_runtime::semantics::insert_pass_semantics(
                &mut uniform_semantics,
                &mut texture_semantics,
                &details.0,
            )
        }
        librashader_runtime::semantics::insert_lut_semantics(
            textures,
            &mut uniform_semantics,
            &mut texture_semantics,
        );

        let semantics = ShaderSemantics {
            uniform_semantics,
            texture_semantics,
        };

        Ok((passes, semantics))
    }

    fn init_passes(
        vulkan: &VulkanDevice,
        passes: Vec<ShaderPassMeta>,
        semantics: &ShaderSemantics,
        frames_in_flight: u32,
    ) -> error::Result<Box<[FilterPass]>> {
        let mut filters = Vec::new();
        let frames_in_flight = std::cmp::max(1, frames_in_flight);

        // initialize passes
        for (index, (config, source, mut reflect)) in passes.into_iter().enumerate() {
            let reflection = reflect.reflect(index, semantics)?;
            let spirv_words = reflect.compile(None)?;

            let ubo_size = reflection
                .ubo
                .as_ref()
                .map(|ubo| ubo.size as usize)
                .unwrap_or(0);
            let uniform_storage = UniformStorage::new(
                ubo_size,
                reflection
                    .push_constant
                    .as_ref()
                    .map(|push| push.size as usize)
                    .unwrap_or(0),
            );

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

            let graphics_pipeline = VulkanGraphicsPipeline::new(
                &vulkan.device,
                &vulkan.pipeline_cache,
                &spirv_words,
                &reflection,
                frames_in_flight,
            )?;

            let ubo_ring = VkUboRing::new(
                &vulkan.device,
                &vulkan.memory_properties,
                frames_in_flight as usize,
                ubo_size,
            )?;
            // shader_vulkan: 2026
            filters.push(FilterPass {
                device: vulkan.device.clone(),
                reflection,
                // compiled: spirv_words,
                uniform_storage,
                uniform_bindings,
                source,
                config,
                graphics_pipeline,
                ubo_ring,
                frames_in_flight,
            });
        }

        Ok(filters.into_boxed_slice())
    }

    fn load_luts(
        vulkan: &VulkanDevice,
        textures: &[TextureConfig],
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
        let mut luts = FxHashMap::default();

        let command_pool = unsafe {
            vulkan.device.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .build(),
                None,
            )?
        };

        let command_buffer = unsafe {
            // panic safety: command buffer count = 1
            vulkan.device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1)
                    .build(),
            )?[0]
        };

        unsafe {
            vulkan.device.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                    .build(),
            )?
        }

        for (index, texture) in textures.iter().enumerate() {
            let image = Image::load(&texture.path, UVDirection::TopLeft)?;

            let texture = LutTexture::new(vulkan, command_buffer, image, texture)?;
            luts.insert(index, texture);
        }

        unsafe {
            vulkan.device.end_command_buffer(command_buffer)?;

            let buffers = [command_buffer];
            let submits = [vk::SubmitInfo::builder().command_buffers(&buffers).build()];

            vulkan
                .device
                .queue_submit(vulkan.queue, &submits, vk::Fence::null())?;
            vulkan.device.queue_wait_idle(vulkan.queue)?;

            vulkan.device.free_command_buffers(command_pool, &buffers);

            vulkan.device.destroy_command_pool(command_pool, None);
        }
        Ok(luts)
    }

    fn init_history(
        vulkan: &VulkanDevice,
        filters: &[FilterPass],
    ) -> error::Result<(VecDeque<OwnedImage>, Box<[Option<InputImage>]>)> {
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
        let mut images = Vec::with_capacity(required_images);
        images.resize_with(required_images, || {
            OwnedImage::new(&vulkan, Size::new(1, 1), ImageFormat::R8G8B8A8Unorm, 1)
        });

        let images: error::Result<Vec<OwnedImage>> = images.into_iter().collect();
        let images = VecDeque::from(images?);

        let mut image_views = Vec::new();
        image_views.resize_with(required_images, || None);

        Ok((images, image_views.into_boxed_slice()))
    }

    // image must be in SHADER_READ_OPTIMAL
    pub fn push_history(
        &mut self,
        input: &VulkanImage,
        cmd: vk::CommandBuffer,
        count: usize,
    ) -> error::Result<()> {
        if let Some(mut back) = self.history_framebuffers.pop_back() {
            if back.image.size != input.size
                || (input.format != vk::Format::UNDEFINED && input.format != back.image.format)
            {
                // eprintln!("[history] resizing");
                // old back will get dropped.. do we need to defer?
                let old_back = std::mem::replace(
                    &mut back,
                    OwnedImage::new(&self.vulkan, input.size, input.format.into(), 1)?,
                );
                self.intermediates[count % self.intermediates.len()].dispose_owned(old_back);
            }

            unsafe {
                util::vulkan_image_layout_transition_levels(
                    &self.vulkan.device,
                    cmd,
                    input.image,
                    1,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    vk::AccessFlags::SHADER_READ,
                    vk::AccessFlags::TRANSFER_READ,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::QUEUE_FAMILY_IGNORED,
                    vk::QUEUE_FAMILY_IGNORED,
                );

                back.copy_from(cmd, &input, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);

                util::vulkan_image_layout_transition_levels(
                    &self.vulkan.device,
                    cmd,
                    input.image,
                    1,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    vk::AccessFlags::TRANSFER_READ,
                    vk::AccessFlags::SHADER_READ,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::QUEUE_FAMILY_IGNORED,
                    vk::QUEUE_FAMILY_IGNORED,
                );
            }

            self.history_framebuffers.push_front(back)
        }

        Ok(())
    }
    /// Records shader rendering commands to the provided command buffer.
    ///
    /// * The input image must be in the `VK_SHADER_READ_ONLY_OPTIMAL`.
    /// * The output image must be in `VK_COLOR_ATTACHMENT_OPTIMAL`.
    ///
    /// librashader **will not** create a pipeline barrier for the final pass. The output image will
    /// remain in `VK_COLOR_ATTACHMENT_OPTIMAL` after all shader passes. The caller must transition
    /// the output image to the final layout.
    pub fn frame(
        &mut self,
        count: usize,
        viewport: &Viewport<VulkanImage>,
        input: &VulkanImage,
        cmd: vk::CommandBuffer,
        options: Option<FrameOptions>,
    ) -> error::Result<()> {
        let mut intermediates = &mut self.intermediates[count % self.intermediates.len()];
        intermediates.dispose();

        // limit number of passes to those enabled.
        let passes = &mut self.passes[0..self.common.config.passes_enabled];

        if let Some(options) = &options {
            if options.clear_history {
                for history in &mut self.history_framebuffers {
                    history.clear(cmd);
                }
            }
        }

        if passes.is_empty() {
            return Ok(());
        }

        let original_image_view = unsafe {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(input.image)
                .format(input.format)
                .view_type(vk::ImageViewType::TYPE_2D)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .level_count(1)
                        .layer_count(1)
                        .build(),
                )
                .components(
                    vk::ComponentMapping::builder()
                        .r(vk::ComponentSwizzle::R)
                        .g(vk::ComponentSwizzle::G)
                        .b(vk::ComponentSwizzle::B)
                        .a(vk::ComponentSwizzle::A)
                        .build(),
                )
                .build();

            self.vulkan.device.create_image_view(&create_info, None)?
        };

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
            image: input.clone(),
            image_view: original_image_view,
            wrap_mode,
            filter_mode: filter,
            mip_filter: filter,
        };

        let mut source = &original;

        // swap output and feedback **before** recording command buffers
        std::mem::swap(
            &mut self.output_framebuffers,
            &mut self.feedback_framebuffers,
        );

        // rescale render buffers to ensure all bindings are valid.
        for (index, pass) in passes.iter_mut().enumerate() {
            self.output_framebuffers[index].scale(
                pass.config.scaling.clone(),
                pass.get_format(),
                &viewport.output.size,
                &original,
                source,
                // todo: need to check **next**
                pass.config.mipmap_input,
                None,
            )?;

            self.feedback_framebuffers[index].scale(
                pass.config.scaling.clone(),
                pass.get_format(),
                &viewport.output.size,
                &original,
                source,
                // todo: need to check **next**
                pass.config.mipmap_input,
                None,
            )?;

            // refresh inputs
            self.common.feedback_inputs[index] = Some(
                self.feedback_framebuffers[index]
                    .as_input(pass.config.filter, pass.config.wrap_mode),
            );
            self.common.output_inputs[index] = Some(
                self.output_framebuffers[index].as_input(pass.config.filter, pass.config.wrap_mode),
            );
        }

        let passes_len = passes.len();
        let (pass, last) = passes.split_at_mut(passes_len - 1);

        let frame_direction = options.map(|f| f.frame_direction).unwrap_or(1);

        for (index, pass) in pass.iter_mut().enumerate() {
            let target = &self.output_framebuffers[index];
            let out = RenderTarget {
                x: 0.0,
                y: 0.0,
                mvp: DEFAULT_MVP,
                output: OutputImage::new(&self.vulkan, target.image.clone())?,
            };

            pass.draw(
                cmd,
                index,
                &self.common,
                if pass.config.frame_count_mod > 0 {
                    count % pass.config.frame_count_mod as usize
                } else {
                    count
                } as u32,
                frame_direction,
                viewport,
                &original,
                &source,
                &out,
            )?;

            if target.max_miplevels > 1 && !self.disable_mipmaps {
                target.generate_mipmaps_and_end_pass(cmd);
            } else {
                out.output.end_pass(cmd);
            }

            source = &self.common.output_inputs[index].as_ref().unwrap();
            intermediates.dispose_outputs(out.output);
        }

        // try to hint the optimizer
        assert_eq!(last.len(), 1);
        if let Some(pass) = last.iter_mut().next() {
            // source.filter_mode = pass.config.filter;
            // source.mip_filter = pass.config.filter;

            let out = RenderTarget {
                x: viewport.x,
                y: viewport.y,
                mvp: viewport.mvp.unwrap_or(DEFAULT_MVP),
                output: OutputImage::new(&self.vulkan, viewport.output.clone())?,
            };

            pass.draw(
                cmd,
                passes_len - 1,
                &self.common,
                count as u32,
                0,
                viewport,
                &original,
                source,
                &out,
            )?;

            intermediates.dispose_outputs(out.output);
        }

        self.push_history(input, cmd, count)?;
        Ok(())
    }
}
