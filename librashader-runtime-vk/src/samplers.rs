use crate::error;
use ash::vk;
use librashader_common::{FilterMode, WrapMode};
use rustc_hash::FxHashMap;

pub struct VulkanSampler {
    pub handle: vk::Sampler,
    device: ash::Device,
}

impl VulkanSampler {
    pub fn new(
        device: &ash::Device,
        wrap: WrapMode,
        filter: FilterMode,
        mipmap: FilterMode,
    ) -> error::Result<VulkanSampler> {
        let create_info = vk::SamplerCreateInfo::builder()
            .mip_lod_bias(0.0)
            .max_anisotropy(1.0)
            .compare_enable(false)
            .min_lod(0.0)
            .max_lod(vk::LOD_CLAMP_NONE)
            .unnormalized_coordinates(false)
            .border_color(vk::BorderColor::FLOAT_TRANSPARENT_BLACK)
            .mag_filter(filter.into())
            .min_filter(filter.into())
            .mipmap_mode(mipmap.into())
            .address_mode_u(wrap.into())
            .address_mode_v(wrap.into())
            .address_mode_w(wrap.into())
            .build();

        let sampler = unsafe { device.create_sampler(&create_info, None)? };

        Ok(VulkanSampler {
            handle: sampler,
            device: device.clone(),
        })
    }
}

impl Drop for VulkanSampler {
    fn drop(&mut self) {
        if self.handle != vk::Sampler::null() {
            unsafe {
                self.device.destroy_sampler(self.handle, None);
            }
        }
    }
}

pub struct SamplerSet {
    // todo: may need to deal with differences in mip filter.
    samplers: FxHashMap<(WrapMode, FilterMode, FilterMode), VulkanSampler>,
}

impl SamplerSet {
    pub fn get(&self, wrap: WrapMode, filter: FilterMode, mipmap: FilterMode) -> &VulkanSampler {
        // eprintln!("{wrap}, {filter}, {mip}");
        self.samplers.get(&(wrap, filter, mipmap)).unwrap()
    }

    pub fn new(device: &ash::Device) -> error::Result<SamplerSet> {
        let mut samplers = FxHashMap::default();
        let wrap_modes = &[
            WrapMode::ClampToBorder,
            WrapMode::ClampToEdge,
            WrapMode::Repeat,
            WrapMode::MirroredRepeat,
        ];
        for wrap_mode in wrap_modes {
            samplers.insert(
                (*wrap_mode, FilterMode::Linear, FilterMode::Linear),
                VulkanSampler::new(device, *wrap_mode, FilterMode::Linear, FilterMode::Linear)?,
            );
            samplers.insert(
                (*wrap_mode, FilterMode::Linear, FilterMode::Nearest),
                VulkanSampler::new(device, *wrap_mode, FilterMode::Linear, FilterMode::Nearest)?,
            );

            samplers.insert(
                (*wrap_mode, FilterMode::Nearest, FilterMode::Nearest),
                VulkanSampler::new(device, *wrap_mode, FilterMode::Nearest, FilterMode::Nearest)?,
            );
            samplers.insert(
                (*wrap_mode, FilterMode::Nearest, FilterMode::Linear),
                VulkanSampler::new(device, *wrap_mode, FilterMode::Nearest, FilterMode::Linear)?,
            );
        }

        Ok(SamplerSet { samplers })
    }
}
