use crate::parameters::RuntimeParameters;
use crate::uniforms::{BindUniform, NoUniformBinder, UniformStorage};
use bit_set::BitSet;
use librashader_common::map::{FastHashMap, ShortString};
use librashader_common::Size;
use librashader_preprocess::ShaderParameter;
use librashader_reflect::reflect::semantics::{
    BindingMeta, MemberOffset, Semantic, TextureBinding, TextureSemantics, UniformBinding,
    UniformMeta, UniqueSemantics,
};
use num_traits::Zero;
use std::ops::{Deref, DerefMut};

/// Trait for input textures used during uniform binding,
pub trait TextureInput {
    /// Gets the size of this input texture.
    fn size(&self) -> Size<u32>;
}

/// A uniform member offset with context that needs to be resolved.
pub trait ContextOffset<H, C, D = ()>
where
    H: BindUniform<C, f32, D>,
    H: BindUniform<C, u32, D>,
    H: BindUniform<C, i32, D>,
    H: for<'a> BindUniform<C, &'a [f32; 3], D>,
    H: for<'a> BindUniform<C, &'a [f32; 4], D>,
    H: for<'a> BindUniform<C, &'a [f32; 16], D>,
{
    /// Gets the `MemberOffset` part of the offset.
    fn offset(&self) -> MemberOffset;

    /// Gets the context part of the offset.
    fn context(&self) -> C;
}

impl<D, H> ContextOffset<H, Option<()>, D> for MemberOffset
where
    H: BindUniform<Option<()>, f32, D>,
    H: BindUniform<Option<()>, u32, D>,
    H: BindUniform<Option<()>, i32, D>,
    H: for<'a> BindUniform<Option<()>, &'a [f32; 3], D>,
    H: for<'a> BindUniform<Option<()>, &'a [f32; 4], D>,
    H: for<'a> BindUniform<Option<()>, &'a [f32; 16], D>,
{
    fn offset(&self) -> MemberOffset {
        *self
    }

    fn context(&self) -> Option<()> {
        None
    }
}

/// Inputs to binding semantics
pub struct UniformInputs<'a> {
    /// MVP
    pub mvp: &'a [f32; 16],
    /// FrameCount
    pub frame_count: u32,
    /// Rotation
    pub rotation: u32,
    /// TotalSubFrames
    pub total_subframes: u32,
    /// CurrentSubFrame
    pub current_subframe: u32,
    /// FrameDirection
    pub frame_direction: i32,
    /// OriginalAspectRatio (need to normalize)
    pub aspect_ratio: f32,
    /// OriginalFPS
    pub frames_per_second: f32,
    /// FrameTimeDelta
    pub frametime_delta: u32,
    /// OutputSize
    pub framebuffer_size: Size<u32>,
    /// FinalViewportSize
    pub viewport_size: Size<u32>,
    /// Grouped HDR inputs.
    pub hdr_inputs: HdrUniformInputs,
    /// Grouped sensor (gyroscope/accelerometer) inputs.
    pub sensor_inputs: SensorUniformInputs,
}

/// Three-axis sensor readings bound to the shader as `vec3` uniforms.
pub struct SensorUniformInputs {
    /// Gyroscope (x, y, z)
    pub gyroscope: [f32; 3],
    /// Accelerometer (x, y, z)
    pub accelerometer: [f32; 3],
    /// AccelerometerRest (Device position at launch / resume) (x, y, z)
    pub accelerometer_rest: [f32; 3],
}

pub struct HdrUniformInputs {
    /// HDRMode
    pub color_space: librashader_common::ColorSpace,
    /// BrightnessNits
    pub brightness_nits: f32,
    /// ExpandGamut
    pub expand_gamut: u32,
}

/// Trait that abstracts binding of semantics to shader uniforms.
pub trait BindSemantics<H = NoUniformBinder, C = Option<()>, U = Box<[u8]>, P = Box<[u8]>>
where
    C: Copy,
    U: Deref<Target = [u8]> + DerefMut,
    P: Deref<Target = [u8]> + DerefMut,
    H: BindUniform<C, f32, Self::DeviceContext>,
    H: BindUniform<C, u32, Self::DeviceContext>,
    H: BindUniform<C, i32, Self::DeviceContext>,
    H: for<'b> BindUniform<C, &'b [f32; 3], Self::DeviceContext>,
    H: for<'b> BindUniform<C, &'b [f32; 4], Self::DeviceContext>,
    H: for<'b> BindUniform<C, &'b [f32; 16], Self::DeviceContext>,
{
    /// The type of the input texture used for semantic binding.
    type InputTexture: TextureInput;

    /// The set of texture samplers available.
    type SamplerSet;

    /// The descriptor set or object that holds sampler and texture bindings.
    type DescriptorSet<'a>;

    /// The device context containing the state of the graphics processor.
    type DeviceContext;

    /// The type of uniform offsets to use.
    type UniformOffset: ContextOffset<H, C, Self::DeviceContext>;

    /// Bind a texture to the input descriptor set
    fn bind_texture<'a>(
        descriptors: &mut Self::DescriptorSet<'a>,
        samplers: &Self::SamplerSet,
        binding: &TextureBinding,
        texture: &Self::InputTexture,
        device: &Self::DeviceContext,
    );

    #[allow(clippy::too_many_arguments)]
    /// Write uniform and texture semantics to the provided storages.
    fn bind_semantics<'a>(
        device: &Self::DeviceContext,
        sampler_set: &Self::SamplerSet,
        uniform_storage: &mut UniformStorage<H, C, U, P, Self::DeviceContext>,
        descriptor_set: &mut Self::DescriptorSet<'a>,
        uniform_inputs: UniformInputs<'_>,
        original: &Self::InputTexture,
        source: &Self::InputTexture,
        uniform_bindings: &FastHashMap<UniformBinding, Self::UniformOffset>,
        texture_meta: &FastHashMap<Semantic<TextureSemantics>, TextureBinding>,
        pass_outputs: impl Iterator<Item = Option<impl AsRef<Self::InputTexture>>>,
        pass_feedback: impl Iterator<Item = Option<impl AsRef<Self::InputTexture>>>,
        original_history: impl Iterator<Item = Option<impl AsRef<Self::InputTexture>>>,
        lookup_textures: impl Iterator<Item = (usize, impl AsRef<Self::InputTexture>)>,
        parameter_defaults: &FastHashMap<ShortString, ShaderParameter>,
        runtime_parameters: &RuntimeParameters,
    ) {
        let runtime_parameters = runtime_parameters.parameters.load();
        // Bind MVP
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::MVP.into()) {
            uniform_storage.bind_mat4(
                offset.offset(),
                uniform_inputs.mvp,
                offset.context(),
                device,
            );
        }

        // Bind OutputSize
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::Output.into()) {
            uniform_storage.bind_vec4(
                offset.offset(),
                uniform_inputs.framebuffer_size,
                offset.context(),
                device,
            );
        }

        // bind FinalViewportSize
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::FinalViewport.into()) {
            uniform_storage.bind_vec4(
                offset.offset(),
                uniform_inputs.viewport_size,
                offset.context(),
                device,
            );
        }

        // bind FrameCount
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::FrameCount.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                uniform_inputs.frame_count,
                offset.context(),
                device,
            );
        }

        // bind FrameDirection
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::FrameDirection.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                uniform_inputs.frame_direction,
                offset.context(),
                device,
            );
        }

        // bind Rotation
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::Rotation.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                uniform_inputs.rotation,
                offset.context(),
                device,
            );
        }

        // bind TotalSubFrames
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::TotalSubFrames.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                uniform_inputs.total_subframes,
                offset.context(),
                device,
            );
        }

        // bind CurrentSubFrames
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::CurrentSubFrame.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                uniform_inputs.current_subframe,
                offset.context(),
                device,
            );
        }

        // bind OriginalFPS
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::OriginalFPS.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                uniform_inputs.frames_per_second,
                offset.context(),
                device,
            );
        }

        // bind FrameTimeDelta
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::FrameTimeDelta.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                uniform_inputs.frametime_delta,
                offset.context(),
                device,
            );
        }

        let mut aspect_ratio = uniform_inputs.aspect_ratio;
        if aspect_ratio.is_zero() {
            aspect_ratio = original.size().aspect_ratio();
        }

        // bind OriginalAspect
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::OriginalAspect.into()) {
            uniform_storage.bind_scalar(offset.offset(), aspect_ratio, offset.context(), device);
        }

        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::OriginalAspectRotated.into()) {
            let rotated_aspect = if uniform_inputs.rotation == 1 || uniform_inputs.rotation == 3 {
                1.0f32 / aspect_ratio
            } else {
                aspect_ratio
            };

            uniform_storage.bind_scalar(offset.offset(), rotated_aspect, offset.context(), device);
        }

        let hdr = uniform_inputs.hdr_inputs;
        // bind HDRMode
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::HDRMode.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                hdr.color_space as u32,
                offset.context(),
                device,
            );
        }

        // bind BrightnessNits
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::BrightnessNits.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                hdr.brightness_nits,
                offset.context(),
                device,
            );
        }

        // bind ExpandGamut
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::ExpandGamut.into()) {
            uniform_storage.bind_scalar(
                offset.offset(),
                hdr.expand_gamut,
                offset.context(),
                device,
            );
        }

        // Scanlines, InverseTonemap, HDR10, and SubpixelLayout are RA-internal
        // hdr.frag uniforms that librashader does not expose for user
        // configuration (RA drives them from its own swapchain mastering
        // pipeline). Bind 0 whenever a shader declares them so the slot is
        // deterministic.
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::Scanlines.into()) {
            uniform_storage.bind_scalar(offset.offset(), 0.0f32, offset.context(), device);
        }
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::InverseTonemap.into()) {
            uniform_storage.bind_scalar(offset.offset(), 0.0f32, offset.context(), device);
        }
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::HDR10.into()) {
            uniform_storage.bind_scalar(offset.offset(), 0.0f32, offset.context(), device);
        }
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::SubpixelLayout.into()) {
            uniform_storage.bind_scalar(offset.offset(), 0u32, offset.context(), device);
        }

        let sensors = uniform_inputs.sensor_inputs;
        // bind Gyroscope
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::Gyroscope.into()) {
            uniform_storage.bind_vec3(offset.offset(), sensors.gyroscope, offset.context(), device);
        }

        // bind Accelerometer
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::Accelerometer.into()) {
            uniform_storage.bind_vec3(
                offset.offset(),
                sensors.accelerometer,
                offset.context(),
                device,
            );
        }

        // bind AccelerometerRest
        if let Some(offset) = uniform_bindings.get(&UniqueSemantics::AccelerometerRest.into()) {
            uniform_storage.bind_vec3(
                offset.offset(),
                sensors.accelerometer_rest,
                offset.context(),
                device,
            );
        }

        // bind Original sampler
        if let Some(binding) = texture_meta.get(&TextureSemantics::Original.semantics(0)) {
            Self::bind_texture(descriptor_set, sampler_set, binding, original, device);
        }

        // bind OriginalSize
        if let Some(offset) = uniform_bindings.get(&TextureSemantics::Original.semantics(0).into())
        {
            uniform_storage.bind_vec4(offset.offset(), original.size(), offset.context(), device);
        }

        // bind Source sampler
        if let Some(binding) = texture_meta.get(&TextureSemantics::Source.semantics(0)) {
            Self::bind_texture(descriptor_set, sampler_set, binding, source, device);
        }

        // bind SourceSize
        if let Some(offset) = uniform_bindings.get(&TextureSemantics::Source.semantics(0).into()) {
            uniform_storage.bind_vec4(offset.offset(), source.size(), offset.context(), device);
        }

        // OriginalHistory0 aliases OriginalHistory

        // bind OriginalHistory0 sampler
        if let Some(binding) = texture_meta.get(&TextureSemantics::OriginalHistory.semantics(0)) {
            Self::bind_texture(descriptor_set, sampler_set, binding, original, device);
        }

        // bind OriginalHistory0Size
        if let Some(offset) =
            uniform_bindings.get(&TextureSemantics::OriginalHistory.semantics(0).into())
        {
            uniform_storage.bind_vec4(offset.offset(), original.size(), offset.context(), device);
        }

        // bind OriginalHistory1-..
        for (index, history) in original_history.enumerate() {
            let Some(history) = history else {
                continue;
            };

            let history = history.as_ref();

            if let Some(binding) =
                texture_meta.get(&TextureSemantics::OriginalHistory.semantics(index + 1))
            {
                Self::bind_texture(descriptor_set, sampler_set, binding, history, device);
            }

            if let Some(offset) = uniform_bindings.get(
                &TextureSemantics::OriginalHistory
                    .semantics(index + 1)
                    .into(),
            ) {
                uniform_storage.bind_vec4(
                    offset.offset(),
                    history.size(),
                    offset.context(),
                    device,
                );
            }
        }

        // bind PassOutput0..
        // The caller should be responsible for limiting this up to
        // pass_index
        for (index, output) in pass_outputs.enumerate() {
            let Some(output) = output else {
                continue;
            };

            let output = output.as_ref();

            if let Some(binding) = texture_meta.get(&TextureSemantics::PassOutput.semantics(index))
            {
                Self::bind_texture(descriptor_set, sampler_set, binding, output, device);
            }

            if let Some(offset) =
                uniform_bindings.get(&TextureSemantics::PassOutput.semantics(index).into())
            {
                uniform_storage.bind_vec4(offset.offset(), output.size(), offset.context(), device);
            }
        }

        // bind PassFeedback0..
        for (index, feedback) in pass_feedback.enumerate() {
            let Some(output) = feedback else {
                continue;
            };

            let feedback = output.as_ref();

            if let Some(binding) =
                texture_meta.get(&TextureSemantics::PassFeedback.semantics(index))
            {
                Self::bind_texture(descriptor_set, sampler_set, binding, feedback, device);
            }

            if let Some(offset) =
                uniform_bindings.get(&TextureSemantics::PassFeedback.semantics(index).into())
            {
                uniform_storage.bind_vec4(
                    offset.offset(),
                    feedback.size(),
                    offset.context(),
                    device,
                );
            }
        }

        // bind User parameters
        for (id, offset) in uniform_bindings
            .iter()
            .filter_map(|(binding, value)| match binding {
                UniformBinding::Parameter(id) => Some((id, value)),
                _ => None,
            })
        {
            let default = parameter_defaults.get(id).map_or(0f32, |f| f.initial);

            let value = *runtime_parameters.get(id).unwrap_or(&default);

            uniform_storage.bind_scalar(offset.offset(), value, offset.context(), device);
        }

        // bind luts
        for (index, lut) in lookup_textures {
            let lut = lut.as_ref();
            if let Some(binding) = texture_meta.get(&TextureSemantics::User.semantics(index)) {
                Self::bind_texture(descriptor_set, sampler_set, binding, lut, device);
            }

            if let Some(offset) =
                uniform_bindings.get(&TextureSemantics::User.semantics(index).into())
            {
                uniform_storage.bind_vec4(offset.offset(), lut.size(), offset.context(), device);
            }
        }
    }
}

#[derive(Debug)]
pub struct BindingRequirements {
    /// The number of `OriginalHistory` frames that must be kept.
    pub(crate) required_history: usize,
    /// Whether the final pass's output must be retained for feedback.
    pub(crate) uses_final_pass_as_feedback: bool,
    /// Pass indices whose output is referenced as `PassFeedback`.
    /// Always a subset of `retained_output_mask`.
    pub(crate) feedback_mask: BitSet,
    /// Pass indices whose output must occupy a dedicated framebuffer for the whole
    /// frame. These passes are referenced via `PassFeedback`, or via `PassOutput`
    /// beyond their immediate successor.
    pub(crate) retained_output_mask: BitSet,
}

/// Trait for objects that can be used to create a binding map.
pub trait BindingUtil {
    /// Create the uniform binding map with the given reflection information.
    fn create_binding_map<T>(
        &self,
        f: impl Fn(&dyn UniformMeta) -> T,
    ) -> FastHashMap<UniformBinding, T>;

    /// Calculate the number of required images for history.
    fn calculate_requirements<'a>(pass_meta: impl Iterator<Item = &'a Self>) -> BindingRequirements
    where
        Self: 'a;
}

impl BindingUtil for BindingMeta {
    fn create_binding_map<T>(
        &self,
        f: impl Fn(&dyn UniformMeta) -> T,
    ) -> FastHashMap<UniformBinding, T> {
        let mut uniform_bindings = FastHashMap::default();
        for param in self.parameter_meta.values() {
            uniform_bindings.insert(UniformBinding::Parameter(param.id.clone()), f(param));
        }

        for (semantics, param) in &self.unique_meta {
            uniform_bindings.insert(UniformBinding::SemanticVariable(*semantics), f(param));
        }

        for (semantics, param) in &self.texture_size_meta {
            uniform_bindings.insert(UniformBinding::TextureSize(*semantics), f(param));
        }

        uniform_bindings
    }

    fn calculate_requirements<'a>(pass_meta: impl Iterator<Item = &'a Self>) -> BindingRequirements
    where
        Self: 'a,
    {
        let mut required_images = 0;

        let mut len: i64 = 0;
        let mut latest_feedback_pass: i64 = -1;

        let mut feedback_mask = BitSet::new();
        let mut retained_output_mask = BitSet::new();

        for (pass_index, pass) in pass_meta.enumerate() {
            len += 1;

            // If a shader uses history size, but not history, we still need to keep the texture.
            let history_texture_max_index = pass
                .texture_meta
                .keys()
                .filter(|semantics| semantics.semantics == TextureSemantics::OriginalHistory)
                .map(|semantic| semantic.index)
                .fold(0, std::cmp::max);
            let history_texture_size_max_index = pass
                .texture_size_meta
                .keys()
                .filter(|semantics| semantics.semantics == TextureSemantics::OriginalHistory)
                .map(|semantic| semantic.index)
                .fold(0, std::cmp::max);

            // A pass output referenced by a future pass as PassFeedback needs its
            // previous-frame output preserved.
            for semantic in pass
                .texture_meta
                .keys()
                .chain(pass.texture_size_meta.keys())
                .filter(|semantics| semantics.semantics == TextureSemantics::PassFeedback)
            {
                feedback_mask.insert(semantic.index);
                latest_feedback_pass = std::cmp::max(latest_feedback_pass, semantic.index as i64);
            }

            // PassOutput read beyond the immediate successor must be retained for the frame.
            // The immediate-successor (`Source`-equivalent, index == pass_index - 1) case is
            // already kept as the previous frame.
            for semantic in pass
                .texture_meta
                .keys()
                .chain(pass.texture_size_meta.keys())
                .filter(|semantics| semantics.semantics == TextureSemantics::PassOutput)
            {
                // overflow safe
                if semantic.index + 1 < pass_index {
                    retained_output_mask.insert(semantic.index);
                }
            }

            required_images = std::cmp::max(required_images, history_texture_max_index);
            required_images = std::cmp::max(required_images, history_texture_size_max_index);
        }

        // Feedback needs a dedicated image to double-buffer against.
        retained_output_mask.union_with(&feedback_mask);

        let uses_feedback = if latest_feedback_pass.is_negative() {
            false
        } else {
            // Technically = but we can be permissive here

            // account for off by 1
            latest_feedback_pass + 1 >= len
        };

        BindingRequirements {
            required_history: required_images,
            uses_final_pass_as_feedback: uses_feedback,
            feedback_mask,
            retained_output_mask,
        }
    }
}

#[macro_export]
macro_rules! impl_default_frame_options {
    ($ty:ident) => {
        /// Options for each frame.
        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct $ty {
            /// Whether or not to clear the history buffers.
            pub clear_history: bool,
            /// The direction of rendering.
            /// -1 indicates that the frames are played in reverse order.
            pub frame_direction: i32,
            /// The rotation of the output. 0 = 0deg, 1 = 90deg, 2 = 180deg, 3 = 270deg.
            pub rotation: u32,
            /// The total number of subframes ran. Default is 1.
            pub total_subframes: u32,
            /// The current sub frame. Default is 1.
            pub current_subframe: u32,
            /// The expected aspect ratio of the source image.
            ///
            /// This can differ from the actual aspect ratio of the source
            /// image.
            ///
            /// The default is 0 which will automatically infer the ratio from the source image.
            pub aspect_ratio: f32,
            /// The original frames per second of the source. Default is 1.
            pub frames_per_second: f32,
            /// Time in milliseconds between the current and previous frame. Default is 0.
            pub frametime_delta: u32,
            /// Target color space bound to the shader `HDRMode` uniform. Must
            /// match the host swapchain color space. Default is
            /// `ColorSpace::Sdr`.
            pub color_space: $crate::__ColorSpace,
            /// HDR SDR reference white in nits, bound to the shader `BrightnessNits` uniform.
            /// Default is 200.0. Only meaningful when the chain's HDR mode is non-zero.
            pub brightness_nits: f32,
            /// Gamut expansion mode bound to the shader `ExpandGamut` uniform. Default is 0.
            pub expand_gamut: u32,
            /// Bound to the shader `Gyroscope` (vec3) uniform. Default is [0, 0, 0].
            pub gyroscope: [f32; 3],
            /// Bound to the shader `Accelerometer` (vec3) uniform. Default is [0, 0, 0].
            pub accelerometer: [f32; 3],
            /// Bound to the shader `AccelerometerRest` (vec3) uniform. Default is [0, 0, 0].
            pub accelerometer_rest: [f32; 3],
        }

        impl Default for $ty {
            fn default() -> Self {
                Self {
                    clear_history: false,
                    frame_direction: 1,
                    rotation: 0,
                    total_subframes: 1,
                    current_subframe: 1,
                    aspect_ratio: 0.0,
                    frametime_delta: 0,
                    frames_per_second: 1.0,
                    color_space: $crate::__ColorSpace::Sdr,
                    brightness_nits: 200.0,
                    expand_gamut: 0,
                    gyroscope: [0.0, 0.0, 0.0],
                    accelerometer: [0.0, 0.0, 0.0],
                    accelerometer_rest: [0.0, 0.0, 0.0],
                }
            }
        }
    };
}
