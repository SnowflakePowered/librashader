use crate::error;
use crate::filter_pass::FilterPass;
use crate::luts::LutTexture;
use crate::vulkan_state::VulkanGraphicsPipeline;
use ash::vk::{CommandPoolCreateFlags, PFN_vkGetInstanceProcAddr, Queue, StaticFn};
use ash::{vk, Device};
use librashader_common::ImageFormat;
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
use crate::samplers::SamplerSet;

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
    // pub(crate) output_framebuffers: Box<[OwnedFramebuffer]>,
    // pub(crate) feedback_framebuffers: Box<[OwnedFramebuffer]>,
    // pub(crate) history_framebuffers: VecDeque<OwnedFramebuffer>,
    // pub(crate) draw_quad: DrawQuad,
}

pub(crate) struct FilterCommon {
    pub(crate) luts: FxHashMap<usize, LutTexture>,
    pub samplers: SamplerSet,
    // pub output_textures: Box<[Option<Texture>]>,
    // pub feedback_textures: Box<[Option<Texture>]>,
    // pub history_textures: Box<[Option<Texture>]>,
    // pub config: FilterMutable,
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
        eprintln!("filters initialized ok.");
        Ok(FilterChainVulkan {
            common: FilterCommon { luts, samplers },
            passes: filters,
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

            let graphics_pipeline = VulkanGraphicsPipeline::new(
                &vulkan.device,
                &vulkan.pipeline_cache,
                &spirv_words,
                &reflection,
                source.format,
                images,
            )?;

            // shader_vulkan: 2026
            filters.push(FilterPass {
                compiled: spirv_words,
                uniform_storage,
                uniform_bindings,
                source,
                config,
                graphics_pipeline,
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
}
