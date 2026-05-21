use librashader_common::{shader_features::ShaderFeatures, ColorSpace};
use librashader_presets::{PresetColorSpace, ShaderPreset};
use librashader_runtime_wgpu::options::FilterChainOptionsWgpu;
use librashader_runtime_wgpu::FilterChainWgpu;

/// Example: load an HDR10-targeting preset (final pass declares
/// `#pragma format A2B10G10R10_UNORM_PACK32`), query `color_space()`
/// to see what swapchain color space the host should request, then construct
/// the filter chain. Non-interactive — uses a headless wgpu device.
#[test]
fn triangle_wgpu_hdr10() {
    // The Sony Megatron HDR preset's final pass declares
    // `#pragma format A2B10G10R10_UNORM_PACK32` — the canonical HDR10
    // (BT.2020 PQ) surface format.
    let hdr_mode = ColorSpace::Hdr10;
    let preset = ShaderPreset::try_parse(
        "../test/shaders_slang/hdr/crt-sony-megatron-default-hdr.slangp",
        ShaderFeatures::NONE,
    )
    .unwrap();
    assert_eq!(preset.color_space().unwrap(), ColorSpace::Hdr10);

    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
                    | wgpu::Features::PIPELINE_CACHE
                    | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                    | wgpu::Features::FLOAT32_FILTERABLE,
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await
            .unwrap();

        let _chain = FilterChainWgpu::load_from_preset(
            preset,
            &device,
            &queue,
            Some(&FilterChainOptionsWgpu {
                force_no_mipmaps: false,
                enable_cache: false,
                adapter_info: None,
                hdr_mode,
            }),
        )
        .unwrap();
    });
}
