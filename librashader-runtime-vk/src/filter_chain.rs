use crate::{error, util};
use crate::filter_pass::FilterPass;
use crate::luts::LutTexture;
use crate::samplers::SamplerSet;
use crate::texture::{OwnedTexture, InputTexture, VulkanImage};
use crate::ubo_ring::VkUboRing;
use crate::vulkan_state::VulkanGraphicsPipeline;
use ash::vk::{CommandPoolCreateFlags, PFN_vkGetInstanceProcAddr, Queue, StaticFn};
use ash::{vk, Device};
use librashader_common::{ImageFormat, Size};
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
use std::error::Error;
use std::path::Path;
use crate::draw_quad::{DrawQuad, VBO_DEFAULT_FINAL, VBO_OFFSCREEN};
use crate::framebuffer::OutputFramebuffer;
use crate::render_target::{DEFAULT_MVP, RenderTarget};
use crate::viewport::Viewport;

pub struct Vulkan {
    // physical_device: vk::PhysicalDevice,
    pub(crate) device: ash::Device,
    // instance: ash::Instance,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    pipeline_cache: vk::PipelineCache,
    pub(crate) memory_properties: vk::PhysicalDeviceMemoryProperties,
}

type ShaderPassMeta = (
    ShaderPassConfig,
    ShaderSource,
    CompilerBackend<impl CompileShader<SpirV, Options = Option<()>, Context = ()> + ReflectShader>,
);

#[derive(Clone)]
pub struct VulkanInfo<'a> {
    // physical_device: &'a vk::PhysicalDevice,
    device: &'a vk::Device,
    instance: &'a vk::Instance,
    queue: &'a vk::Queue,
    memory_properties: &'a vk::PhysicalDeviceMemoryProperties,
    get_instance_proc_addr: PFN_vkGetInstanceProcAddr,
}

impl TryFrom<VulkanInfo<'_>> for Vulkan {
    type Error = Box<dyn Error>;

    fn try_from(vulkan: VulkanInfo) -> Result<Self, Box<dyn Error>> {
        unsafe {
            let instance = ash::Instance::load(
                &StaticFn {
                    get_instance_proc_addr: vulkan.get_instance_proc_addr,
                },
                vulkan.instance.clone(),
            );

            let device = ash::Device::load(instance.fp_v1_0(), vulkan.device.clone());

            let pipeline_cache = unsafe {
                device.create_pipeline_cache(&vk::PipelineCacheCreateInfo::default(), None)?
            };

            // the queue is only used for lut loading.. we may want to select our own queue.
            let command_pool = unsafe {
                device.create_command_pool(
                    &vk::CommandPoolCreateInfo::builder()
                        .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                        .build(),
                    None,
                )?
            };

            Ok(Vulkan {
                device,
                // instance,
                queue: vulkan.queue.clone(),
                command_pool,
                pipeline_cache,
                memory_properties: vulkan.memory_properties.clone(),
            })
        }
    }
}

impl TryFrom<(ash::Device, vk::Queue, vk::PhysicalDeviceMemoryProperties)> for Vulkan {
    type Error = Box<dyn Error>;

    fn try_from(value: (Device, Queue, vk::PhysicalDeviceMemoryProperties)) -> error::Result<Self> {
        unsafe {
            let device = value.0;

            let pipeline_cache = unsafe {
                device.create_pipeline_cache(&vk::PipelineCacheCreateInfo::default(), None)?
            };

            let command_pool = unsafe {
                device.create_command_pool(
                    &vk::CommandPoolCreateInfo::builder()
                        .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                        .build(),
                    None,
                )?
            };

            Ok(Vulkan {
                device,
                queue: value.1,
                command_pool,
                pipeline_cache,
                memory_properties: value.2,
            })
        }
    }
}

pub struct FilterChainVulkan {
    pub(crate) common: FilterCommon,
    pub(crate) passes: Box<[FilterPass]>,
    pub(crate) vulkan: Vulkan,
    pub(crate) output_framebuffers: Box<[OwnedTexture]>,
    // pub(crate) feedback_framebuffers: Box<[OwnedFramebuffer]>,
    // pub(crate) history_framebuffers: VecDeque<OwnedFramebuffer>,
}

pub struct FilterMutable {
    pub(crate) passes_enabled: usize,
    pub(crate) parameters: FxHashMap<String, f32>,
}

pub(crate) struct FilterCommon {
    pub(crate) luts: FxHashMap<usize, LutTexture>,
    pub samplers: SamplerSet,
    pub(crate) draw_quad: DrawQuad,

    pub output_textures: Box<[Option<InputTexture>]>,
    // pub feedback_textures: Box<[Option<Texture>]>,
    // pub history_textures: Box<[Option<Texture>]>,
    pub config: FilterMutable,
    pub device: ash::Device,
}

#[must_use]
pub struct FilterChainFrameIntermediates {
    device: ash::Device,
    framebuffers: Vec<vk::Framebuffer>,
    image_views: Vec<vk::ImageView>
}

impl FilterChainFrameIntermediates {
    pub(crate) fn new(device: &ash::Device) -> Self {
        FilterChainFrameIntermediates {
            device: device.clone(),
            framebuffers: Vec::new(),
            image_views: Vec::new(),
        }
    }

    pub(crate) fn dispose_input(&mut self, input_texture_texture: InputTexture) {
        self.image_views.push(input_texture_texture.image_view);
    }

    pub(crate) fn dispose_outputs(&mut self, output_framebuffer: OutputFramebuffer) {
        // self.framebuffers.push(output_framebuffer.framebuffer);
        self.image_views.push(output_framebuffer.image_view);
    }

    pub(crate) fn dispose_framebuffer(&mut self, framebuffer: vk::Framebuffer) {
        self.framebuffers.push(framebuffer)
    }

    pub(crate) fn dispose_image_views(&mut self, image_view: vk::ImageView) {
        self.image_views.push(image_view)
    }
}

impl Drop for FilterChainFrameIntermediates {
    fn drop(&mut self) {
        for framebuffer in &self.framebuffers {
            if *framebuffer != vk::Framebuffer::null() {
                unsafe {
                    self.device.destroy_framebuffer(*framebuffer, None);
                }
            }
        }

        for image_view in &self.image_views {
            if *image_view != vk::ImageView::null() {
                unsafe {
                    self.device.destroy_image_view(*image_view, None);
                }
            }
        }
    }
}

pub type FilterChainOptionsVulkan = ();

impl FilterChainVulkan {
    /// Load the shader preset at the given path into a filter chain.
    pub fn load_from_path(
        vulkan: impl TryInto<Vulkan, Error = Box<dyn Error>>,
        path: impl AsRef<Path>,
        options: Option<&FilterChainOptionsVulkan>,
    ) -> error::Result<FilterChainVulkan> {
        // load passes from preset
        let preset = ShaderPreset::try_parse(path)?;
        Self::load_from_preset(vulkan, preset, options)
    }

    pub fn load_from_preset(
        vulkan: impl TryInto<Vulkan, Error = Box<dyn Error>>,
        preset: ShaderPreset,
        options: Option<&FilterChainOptionsVulkan>,
    ) -> error::Result<FilterChainVulkan> {
        let (passes, semantics) = FilterChainVulkan::load_preset(preset.shaders, &preset.textures)?;
        let device = vulkan.try_into()?;

        // initialize passes
        let filters = Self::init_passes(&device, passes, &semantics, 3)?;

        let luts = FilterChainVulkan::load_luts(&device, &preset.textures)?;
        let samplers = SamplerSet::new(&device.device)?;

        let mut output_framebuffers = Vec::new();
        output_framebuffers.resize_with(filters.len(), || {
            OwnedTexture::new(
                &device,
                Size::new(1, 1),
                ImageFormat::R8G8B8A8Unorm,
                1
            )
        });

        let output_framebuffers: error::Result<Vec<OwnedTexture>> = output_framebuffers.into_iter().collect();
        let mut output_textures = Vec::new();
        output_textures.resize_with(filters.len(), || None);
        eprintln!("filters initialized ok.");
        Ok(FilterChainVulkan {
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
                output_textures: output_textures.into_boxed_slice()
            },
            passes: filters,
            vulkan: device,
            output_framebuffers: output_framebuffers?.into_boxed_slice(),
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
                eprintln!("[vk] loading {}", &shader.name.display());
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
                Ok::<_, Box<dyn Error>>((shader, source, reflect))
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
        vulkan: &Vulkan,
        passes: Vec<ShaderPassMeta>,
        semantics: &ShaderSemantics,
        images: u32,
    ) -> error::Result<Box<[FilterPass]>> {
        let mut filters = Vec::new();

        // initialize passes
        for (index, (config, mut source, mut reflect)) in passes.into_iter().enumerate() {
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

            // default to something sane
            if source.format == ImageFormat::Unknown {
                source.format = ImageFormat::R8G8B8A8Unorm
            }

            // preset overrides shader
            if let Some(format) = config.get_format_override() {
                source.format = format;
            }

            let graphics_pipeline = VulkanGraphicsPipeline::new(
                &vulkan.device,
                &vulkan.pipeline_cache,
                &spirv_words,
                &reflection,
                source.format,
                images,
            )?;

            let ubo_ring = VkUboRing::new(
                &vulkan.device,
                &vulkan.memory_properties,
                images as usize,
                ubo_size,
            )?;
            // shader_vulkan: 2026
            filters.push(FilterPass {
                device: vulkan.device.clone(),
                reflection,
                compiled: spirv_words,
                uniform_storage,
                uniform_bindings,
                source,
                config,
                graphics_pipeline,
                ubo_ring,
            });
        }

        Ok(filters.into_boxed_slice())
    }

    fn load_luts(
        vulkan: &Vulkan,
        textures: &[TextureConfig],
    ) -> error::Result<FxHashMap<usize, LutTexture>> {
        let mut luts = FxHashMap::default();
        let command_buffer = unsafe {
            // panic safety: command buffer count = 1
            vulkan.device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(vulkan.command_pool)
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

            let texture = LutTexture::new(vulkan, &command_buffer, image, texture)?;
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
            vulkan
                .device
                .free_command_buffers(vulkan.command_pool, &buffers);
        }
        Ok(luts)
    }

    /// Process a frame with the input image.
    ///
    /// When this frame returns, GL_FRAMEBUFFER is bound to 0.
    pub fn frame(
        &mut self,
        count: usize,
        viewport: &Viewport,
        input: &VulkanImage,
        cmd: vk::CommandBuffer,
        options: Option<()>,
    ) -> error::Result<FilterChainFrameIntermediates> {
        // limit number of passes to those enabled.
        let passes = &mut self.passes[0..self.common.config.passes_enabled];

        let mut intermediates = FilterChainFrameIntermediates::new(&self.vulkan.device);
        if passes.is_empty() {
            return Ok(intermediates);
        }


        let original_image_view = unsafe {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(input.image)
                .format(input.format)
                .view_type(vk::ImageViewType::TYPE_2D)
                .subresource_range(vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1)
                    .build())
                .components(vk::ComponentMapping::builder()
                                .r(vk::ComponentSwizzle::R)
                                .g(vk::ComponentSwizzle::G)
                                .b(vk::ComponentSwizzle::B)
                                .a(vk::ComponentSwizzle::A)
                                .build())
                .build();

            self.vulkan.device.create_image_view(&create_info, None)?
        };

        let filter = passes[0].config.filter;
        let wrap_mode = passes[0].config.wrap_mode;

        let original = InputTexture {
            image: input.clone(),
            image_view: original_image_view,
            wrap_mode,
            filter_mode: filter,
            mip_filter: filter,
        };

        let mut source = original.clone();

        // rescale render buffers to ensure all bindings are valid.
        for (index, pass) in passes.iter_mut().enumerate() {
            self.output_framebuffers[index].scale(
                pass.config.scaling.clone(),
                pass.get_format(),
                &viewport.output.size,
                &original,
                &source,
            )?;
        }


        let passes_len = passes.len();
        let (pass, last) = passes.split_at_mut(passes_len - 1);

        for (index, pass) in pass.iter_mut().enumerate() {
            let target = &self.output_framebuffers[index];
            // todo: use proper mode
            // todo: the output framebuffers can only be dropped after command queue submission.

            let out = RenderTarget {
                x: 0.0,
                y: 0.0,
                mvp: DEFAULT_MVP,
                output: OutputFramebuffer::new(&self.vulkan, /*&pass.graphics_pipeline.render_pass,*/
                                               target.image.clone())?,
            };

            pass.draw(cmd, index, &self.common, count as u32, 0, viewport, &original, &source, &out)?;
            // for second to last pass, we want to transition to copy instead.
            out.output.end_pass(cmd);

            source = target.as_input(pass.config.filter, pass.config.wrap_mode)?;
            let prev_frame_output = self.common
                .output_textures[index].replace(source.clone());
            if let Some(prev_frame_output) = prev_frame_output {
                intermediates.dispose_input(prev_frame_output);
            }

            intermediates.dispose_outputs(out.output);
        }

        // try to hint the optimizer
        assert_eq!(last.len(), 1);
        if let Some(pass) = last.iter_mut().next() {
            source.filter_mode = pass.config.filter;
            source.mip_filter = pass.config.filter;

            let out = RenderTarget {
                x: viewport.x,
                y: viewport.y,
                mvp: viewport.mvp.unwrap_or(DEFAULT_MVP),
                output: OutputFramebuffer::new(&self.vulkan,
                                               /*&pass.graphics_pipeline.render_pass,*/
                                               viewport.output.clone())?,
            };

            pass.draw(cmd, passes_len - 1,
                      &self.common,
                      count as u32,
                      0, viewport, &original, &source, &out)?;

            intermediates.dispose_outputs(out.output);
        }
        Ok(intermediates)
    }
}
