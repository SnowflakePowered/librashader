use crate::graphics_pipeline::WgpuGraphicsPipeline;
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::wgsl::NagaWgslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{BindingStage, MemberOffset, TextureBinding, UniformBinding};
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::uniforms::{NoUniformBinder, UniformStorage, UniformStorageAccess};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use wgpu::{BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer, BufferBinding, BufferUsages, RenderPass, ShaderStages, TextureView};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use librashader_common::{Size, Viewport};
use librashader_runtime::binding::{BindSemantics, TextureInput};
use librashader_runtime::quad::QuadType;
use librashader_runtime::render_target::RenderTarget;
use crate::error;
use crate::filter_chain::FilterCommon;
use crate::framebuffer::OutputImage;
use crate::samplers::SamplerSet;
use crate::texture::{InputImage, OwnedImage};

pub struct FilterPass {
    pub device: Arc<wgpu::Device>,
    pub reflection: ShaderReflection,
    pub(crate) compiled: ShaderCompilerOutput<String, NagaWgslContext>,
    pub(crate) uniform_storage: UniformStorage,
    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
    pub graphics_pipeline: WgpuGraphicsPipeline,
    // pub ubo_ring: VkUboRing,
    // pub frames_in_flight: u32,
}

impl TextureInput for InputImage {
    fn size(&self) -> Size<u32> {
        self.image.size().into()
    }
}

impl BindSemantics<NoUniformBinder, Option<()>> for FilterPass {
    type InputTexture = InputImage;
    type SamplerSet = SamplerSet;
    type DescriptorSet<'a> = (
        &'a mut FxHashMap<u32, BindGroupEntry<'a>>,
        &'a mut  FxHashMap<u32, BindGroupEntry<'a>>,
    );
    type DeviceContext = Arc<wgpu::Device>;
    type UniformOffset = MemberOffset;

    #[inline(always)]
    fn bind_texture<'a>(
        descriptors: &mut Self::DescriptorSet<'a>,
        samplers: &Self::SamplerSet,
        binding: &TextureBinding,
        texture: &Self::InputTexture,
        _device: &Self::DeviceContext,
    ) {
        let sampler = samplers.get(texture.wrap_mode, texture.filter_mode, texture.mip_filter);

        let (texture_binding, sampler_binding) = descriptors;
        texture_binding.insert(binding.binding, BindGroupEntry {
            binding: binding.binding,
            resource:BindingResource::TextureView(&texture.view)}
        );
        sampler_binding.insert(binding.binding, BindGroupEntry {
            binding: binding.binding,
            resource: BindingResource::Sampler(&sampler),
        });
    }
}

impl FilterPass {
    pub(crate) fn draw(
        &mut self,
        cmd: &mut wgpu::CommandEncoder,
        pass_index: usize,
        parent: &FilterCommon,
        frame_count: u32,
        frame_direction: i32,
        viewport: &Viewport<OwnedImage>,
        original: &InputImage,
        source: &InputImage,
        output: &RenderTarget<OutputImage>,
        vbo_type: QuadType,
    ) -> error::Result<RenderPass> {

        let mut main_heap = FxHashMap::default();
        let mut sampler_heap = FxHashMap::default();

        self.build_semantics(
            pass_index,
            parent,
            output.mvp,
            frame_count,
            frame_direction,
            output.output.size,
            viewport.output.size,
            original,
            source,
            &mut main_heap,
            &mut sampler_heap,
        );


        let main_buffer: Buffer;
        let pcb_buffer: Buffer;
        if let Some(ubo) = &self.reflection.ubo {
             main_buffer = self.device
                .create_buffer_init(&BufferInitDescriptor {
                    label: Some("ubo buffer"),
                    contents: self.uniform_storage.ubo_slice(),
                    usage: BufferUsages::UNIFORM,
                });

            main_heap.insert(ubo.binding, BindGroupEntry {
                binding: ubo.binding,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &main_buffer,
                    offset: 0,
                    size: None,
                }),
            });
        }

        let mut has_pcb_buffer = false;
        if let Some(pcb) = &self.reflection.push_constant {
            if let Some(binding) = pcb.binding {
                pcb_buffer = self.device
                    .create_buffer_init(&BufferInitDescriptor {
                        label: Some("ubo buffer"),
                        contents: self.uniform_storage.push_slice(),
                        usage: BufferUsages::UNIFORM,
                    });

                main_heap.insert(binding, BindGroupEntry {
                    binding,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &pcb_buffer,
                        offset: 0,
                        size: None,
                    }),
                });
                has_pcb_buffer = true;
            }
        }


        let mut render_pass = self.graphics_pipeline
            .begin_rendering(output, cmd);

        let main_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("main bind group"),
            layout: &self.graphics_pipeline.layout.main_bind_group_layout,
            entries: &main_heap.into_values().collect::<Vec<_>>()
        });

        let sampler_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("sampler bind group"),
            layout: &self.graphics_pipeline.layout.sampler_bind_group_layout,
            entries: &sampler_heap.into_values().collect::<Vec<_>>()
        });

        render_pass.set_bind_group(
            0,
            &main_bind_group,
            &[]
        );

        render_pass.set_bind_group(
            1,
            &sampler_bind_group,
            &[]
        );

        if let Some(push) = &self.reflection.push_constant && !has_pcb_buffer {
            let mut stage_mask = ShaderStages::empty();
            if push.stage_mask.contains(BindingStage::FRAGMENT) {
                stage_mask |= ShaderStages::FRAGMENT;
            }
            if push.stage_mask.contains(BindingStage::VERTEX) {
                stage_mask |= ShaderStages::VERTEX;
            }
            render_pass.set_push_constants(
                stage_mask,
                0,
                self.uniform_storage.push_slice()
            )
        }

        parent.draw_quad.draw_quad(&mut render_pass, vbo_type);

        Ok(render_pass)
    }

    fn build_semantics(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        mvp: &[f32; 16],
        frame_count: u32,
        frame_direction: i32,
        fb_size: Size<u32>,
        viewport_size: Size<u32>,
        original: &InputImage,
        source: &InputImage,
        main_heap: &mut FxHashMap<u32, BindGroupEntry>
        sampler_heap: &mut FxHashMap<u32, BindGroupEntry>
    ) {
        Self::bind_semantics(
            &self.device,
            &parent.samplers,
            &mut self.uniform_storage,
            &mut (main_heap, sampler_heap),
            mvp,
            frame_count,
            frame_direction,
            fb_size,
            viewport_size,
            original,
            source,
            &self.uniform_bindings,
            &self.reflection.meta.texture_meta,
            parent.output_textures[0..pass_index]
                .iter()
                .map(|o| o.as_ref()),
            parent.feedback_textures.iter().map(|o| o.as_ref()),
            parent.history_textures.iter().map(|o| o.as_ref()),
            parent.luts.iter().map(|(u, i)| (*u, i.as_ref())),
            &self.source.parameters,
            &parent.config.parameters,
        );
    }
}
