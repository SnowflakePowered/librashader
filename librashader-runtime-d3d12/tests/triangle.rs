mod hello_triangle;

use crate::hello_triangle::{DXSample, SampleCommandLine};

/// Example: load an HDR-aware preset, query `color_space()` to see
/// what swapchain color space the host should configure, then construct the
/// filter chain with the matching `hdr_mode`. Non-interactive — no window is
/// opened.
#[test]
fn triangle_d3d12_hdr() {
    use librashader_common::{shader_features::ShaderFeatures, ColorSpace};
    use librashader_presets::{PresetColorSpace, ShaderPreset};
    use librashader_runtime_d3d12::options::FilterChainOptionsD3D12;
    use librashader_runtime_d3d12::FilterChainD3D12;
    use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
    use windows::Win32::Graphics::Direct3D12::{D3D12CreateDevice, ID3D12Device};
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory2, IDXGIFactory4};

    let preset =
        ShaderPreset::try_parse("../test/shaders_slang/hdr/hdr.slangp", ShaderFeatures::NONE)
            .unwrap();
    assert_eq!(preset.color_space().unwrap(), ColorSpace::ScRgb);

    // Headless device creation — no swapchain needed.
    let factory: IDXGIFactory4 = unsafe { CreateDXGIFactory2(Default::default()) }.unwrap();
    let adapter = unsafe { factory.EnumAdapters1(0) }.unwrap();

    let mut device: Option<ID3D12Device> = None;
    unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device) }.unwrap();
    let device = device.unwrap();

    let _chain = unsafe {
        FilterChainD3D12::load_from_preset(
            preset,
            &device,
            Some(&FilterChainOptionsD3D12 {
                disable_cache: true,
                ..Default::default()
            }),
        )
    }
    .unwrap();
}

#[test]
fn triangle_d3d12() {
    let sample = hello_triangle::d3d12_hello_triangle::Sample::new(
        // "../test/shaders_slang/sonkun/slot-mask/flat-screen/1080p/test.slangp",
        "../test/shaders_slang/crt/crt-royale.slangp",
        // "../test/basic.slangp",
        // "../test/shaders_slang/handheld/console-border/gbc-lcd-grid-v2.slangp",
        // "../test/Mega_Bezel_Packs/Duimon-Mega-Bezel/Presets/Advanced/Nintendo_GBA_SP/GBA_SP-[ADV]-[LCD-GRID]-[Night].slangp",
        // "../test/shaders_slang/test/feedback.slangp",
        // "../test/shaders_slang/test/history.slangp",
        // "../test/shaders_slang/crt/crt-geom-deluxe.slangp",
        // "../test/slang-shaders/vhs/VHSPro.slangp",
        &SampleCommandLine {
            use_warp_device: false,
        },
    )
    .unwrap();
    hello_triangle::main(sample).unwrap()
}
