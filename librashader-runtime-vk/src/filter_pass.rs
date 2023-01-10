use crate::error;
use crate::filter_chain::FilterCommon;
use crate::rendertarget::RenderTarget;
use crate::samplers::{SamplerSet, VulkanSampler};
use crate::texture::Texture;
use crate::ubo_ring::VkUboRing;
use crate::vulkan_state::VulkanGraphicsPipeline;
use ash::vk;
use librashader_common::{ImageFormat, Size};
use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::ShaderCompilerOutput;
use librashader_reflect::reflect::semantics::{
    MemberOffset, TextureBinding, TextureSemantics, UniformBinding, UniqueSemantics,
};
use librashader_reflect::reflect::ShaderReflection;
use librashader_runtime::uniforms::UniformStorage;
use rustc_hash::FxHashMap;

pub struct FilterPass {
    pub device: ash::Device,
    pub reflection: ShaderReflection,
    pub(crate) compiled: ShaderCompilerOutput<Vec<u32>>,
    pub(crate) uniform_storage: UniformStorage,
    pub uniform_bindings: FxHashMap<UniformBinding, MemberOffset>,
    pub source: ShaderSource,
    pub config: ShaderPassConfig,
    pub graphics_pipeline: VulkanGraphicsPipeline,
    pub ubo_ring: VkUboRing,
}

impl FilterPass {
    #[inline(always)]
    fn bind_texture(
        device: &ash::Device,
        samplers: &SamplerSet,
        descriptor_set: vk::DescriptorSet,
        binding: &TextureBinding,
        texture: &Texture,
    ) {
        let sampler = samplers.get(texture.wrap_mode, texture.filter_mode, texture.mip_filter);
        let image_info = [vk::DescriptorImageInfo::builder()
            .sampler(sampler.handle)
            .image_view(texture.image_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build()];

        let write_desc = [vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(binding.binding)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_info)
            .build()];
        unsafe {
            device.update_descriptor_sets(&write_desc, &[]);
        }
    }

    pub fn get_format(&self) -> ImageFormat {
        let mut fb_format = ImageFormat::R8G8B8A8Unorm;
        if self.config.srgb_framebuffer {
            fb_format = ImageFormat::R8G8B8A8Srgb;
        } else if self.config.float_framebuffer {
            fb_format = ImageFormat::R16G16B16A16Sfloat;
        }
        fb_format
    }

    pub(crate) fn draw(
        &mut self,
        cmd: vk::CommandBuffer,
        pass_index: usize,
        parent: &FilterCommon,
        frame_count: u32,
        frame_direction: i32,
        viewport: &vk::Viewport,
        original: &Texture,
        source: &Texture,
        output: &RenderTarget,
    ) -> error::Result<()> {
        let descriptor = *&self.graphics_pipeline.layout.descriptor_sets[0];

        self.build_semantics(
            pass_index,
            parent,
            &output.mvp,
            frame_count,
            frame_direction,
            output.output.size,
            viewport.into(),
            &descriptor,
            original,
            source,
        );

        if let Some(ubo) = &self.reflection.ubo {
            // shader_vulkan: 2554 (ra uses uses one big buffer)
            // itll be simpler for us if we just use a RingBuffer<vk::Buffer> tbh.
            self.ubo_ring
                .bind_to_descriptor_set(descriptor, ubo.binding, &self.uniform_storage)?;
        }

        Ok(())
    }

    //
    // fn bind_ubo(device: &vk::Device, descriptor: &vk::DescriptorSet, binding: u32, buffer: &vk::Buffer, offset: vk::DeviceSize, range: vk::DeviceSize) {
    //
    // }

    fn build_semantics(
        &mut self,
        pass_index: usize,
        parent: &FilterCommon,
        mvp: &[f32; 16],
        frame_count: u32,
        frame_direction: i32,
        fb_size: Size<u32>,
        viewport_size: Size<u32>,
        descriptor_set: &vk::DescriptorSet,
        original: &Texture,
        source: &Texture,
    ) {
        if let Some(offset) = self.uniform_bindings.get(&UniqueSemantics::MVP.into()) {
            self.uniform_storage.bind_mat4(*offset, mvp, None);
        }

        // bind OutputSize
        if let Some(offset) = self.uniform_bindings.get(&UniqueSemantics::Output.into()) {
            self.uniform_storage.bind_vec4(*offset, fb_size, None);
        }

        // bind FinalViewportSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&UniqueSemantics::FinalViewport.into())
        {
            self.uniform_storage.bind_vec4(*offset, viewport_size, None);
        }

        // bind FrameCount
        if let Some(offset) = self
            .uniform_bindings
            .get(&UniqueSemantics::FrameCount.into())
        {
            self.uniform_storage.bind_scalar(*offset, frame_count, None);
        }

        // bind FrameDirection
        if let Some(offset) = self
            .uniform_bindings
            .get(&UniqueSemantics::FrameDirection.into())
        {
            self.uniform_storage
                .bind_scalar(*offset, frame_direction, None);
        }

        // bind Original sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Original.semantics(0))
        {
            FilterPass::bind_texture(
                &self.device,
                &parent.samplers,
                *descriptor_set,
                binding,
                original,
            );
        }

        // bind OriginalSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::Original.semantics(0).into())
        {
            self.uniform_storage
                .bind_vec4(*offset, original.image.size, None);
        }

        // bind Source sampler
        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::Source.semantics(0))
        {
            // eprintln!("setting source binding to {}", binding.binding);
            FilterPass::bind_texture(
                &self.device,
                &parent.samplers,
                *descriptor_set,
                binding,
                source,
            );
        }

        // bind SourceSize
        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::Source.semantics(0).into())
        {
            self.uniform_storage
                .bind_vec4(*offset, source.image.size, None);
        }

        if let Some(binding) = self
            .reflection
            .meta
            .texture_meta
            .get(&TextureSemantics::OriginalHistory.semantics(0))
        {
            FilterPass::bind_texture(
                &self.device,
                &parent.samplers,
                *descriptor_set,
                binding,
                original,
            );
        }

        if let Some(offset) = self
            .uniform_bindings
            .get(&TextureSemantics::OriginalHistory.semantics(0).into())
        {
            self.uniform_storage
                .bind_vec4(*offset, original.image.size, None);
        }

        // for (index, output) in parent.history_textures.iter().enumerate() {
        //     let Some(output) = output else {
        //         eprintln!("no history");
        //         continue;
        //     };
        //     if let Some(binding) = self
        //         .reflection
        //         .meta
        //         .texture_meta
        //         .get(&TextureSemantics::OriginalHistory.semantics(index + 1))
        //     {
        //         FilterPass::bind_texture(
        //             &self.device,
        //             &parent.samplers,
        //             descriptor_set,
        //             binding,
        //             output,
        //         );
        //     }
        //
        //     if let Some(offset) = self.uniform_bindings.get(
        //         &TextureSemantics::OriginalHistory
        //             .semantics(index + 1)
        //             .into(),
        //     ) {
        //         self.uniform_storage
        //             .bind_vec4(*offset, output.size, None);
        //     }
        // }

        // PassOutput
        // for (index, output) in parent.output_textures[0..pass_index].iter().enumerate() {
        //     let Some(output) = output else {
        //         eprintln!("no passoutput {index}");
        //
        //         continue;
        //     };
        //     if let Some(binding) = self
        //         .reflection
        //         .meta
        //         .texture_meta
        //         .get(&TextureSemantics::PassOutput.semantics(index))
        //     {
        //         FilterPass::bind_texture(
        //             &self.device,
        //             &parent.samplers,
        //             descriptor_set,
        //             binding,
        //             output,
        //         );
        //     }
        //
        //     if let Some(offset) = self
        //         .uniform_bindings
        //         .get(&TextureSemantics::PassOutput.semantics(index).into())
        //     {
        //         self.uniform_storage
        //             .bind_vec4(*offset, output.view.size, None);
        //     }
        // }

        // // PassFeedback
        // for (index, feedback) in parent.feedback_textures.iter().enumerate() {
        //     let Some(feedback) = feedback else {
        //         eprintln!("no passfeedback {index}");
        //         continue;
        //     };
        //     if let Some(binding) = self
        //         .reflection
        //         .meta
        //         .texture_meta
        //         .get(&TextureSemantics::PassFeedback.semantics(index))
        //     {
        //         FilterPass::bind_texture(
        //             &parent.samplers,
        //             &mut textures,
        //             &mut samplers,
        //             binding,
        //             feedback,
        //         );
        //     }
        //
        //     if let Some(offset) = self
        //         .uniform_bindings
        //         .get(&TextureSemantics::PassFeedback.semantics(index).into())
        //     {
        //         self.uniform_storage
        //             .bind_vec4(*offset, feedback.view.size, None);
        //     }
        // }

        // bind float parameters
        for (id, offset) in
            self.uniform_bindings
                .iter()
                .filter_map(|(binding, value)| match binding {
                    UniformBinding::Parameter(id) => Some((id, value)),
                    _ => None,
                })
        {
            let id = id.as_str();

            let default = self
                .source
                .parameters
                .iter()
                .find(|&p| p.id == id)
                .map(|f| f.initial)
                .unwrap_or(0f32);

            let value = *parent.config.parameters.get(id).unwrap_or(&default);

            self.uniform_storage.bind_scalar(*offset, value, None);
        }

        // bind luts
        for (index, lut) in &parent.luts {
            if let Some(binding) = self
                .reflection
                .meta
                .texture_meta
                .get(&TextureSemantics::User.semantics(*index))
            {
                FilterPass::bind_texture(
                    &self.device,
                    &parent.samplers,
                    *descriptor_set,
                    binding,
                    &lut.image,
                );
            }

            if let Some(offset) = self
                .uniform_bindings
                .get(&TextureSemantics::User.semantics(*index).into())
            {
                self.uniform_storage
                    .bind_vec4(*offset, lut.image.image.size, None);
            }
        }

        // (textures, samplers)
    }
}
