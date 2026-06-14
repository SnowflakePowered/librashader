mod hello_triangle;

use librashader_runtime::image::{Image, UVDirection};
use librashader_runtime::parameters::FilterChainParameters;
use librashader_runtime_d3d11::options::FilterChainOptionsD3D11;
use librashader_runtime_d3d11::FilterChainD3D11;

// "../test/slang-shaders/scalefx/scalefx-9x.slangp",
// "../test/slang-shaders/bezel/koko-aio/monitor-bloom.slangp",
// "../test/slang-shaders/presets/crt-geom-ntsc-upscale-sharp.slangp",
// const FILTER_PATH: &str =
//     "../test/slang-shaders/handheld/console-border/gbc-lcd-grid-v2.slangp";
// "../test/null.slangp",
// const FILTER_PATH: &str =
//     "../test/Mega_Bezel_Packs/Duimon-Mega-Bezel/Presets/Advanced/Nintendo_GBA_SP/GBA_SP-[ADV]-[LCD-GRID].slangp";

const FILTER_PATH: &str = "../test/shaders_slang/crt/crt-royale.slangp";

// const FILTER_PATH: &str = "../test/slang-shaders/test/history.slangp";
// const FILTER_PATH: &str = "../test/shaders_slang/test/feedback.slangp";

// const FILTER_PATH: &str = "../test/aspect.slangp";
// const FILTER_PATH: &str = "../test/shaders_slang/sonkun/slot-mask/flat-screen/1080p/test.slangp";

const IMAGE_PATH: &str = "../triangle.png";
#[test]
fn triangle_d3d11_args() {
    let mut args = std::env::args();
    let _ = args.next();
    let _ = args.next();
    let filter = args.next();
    let image = args
        .next()
        .and_then(|f| Image::load(f, UVDirection::TopLeft).ok())
        .or_else(|| Some(Image::load(IMAGE_PATH, UVDirection::TopLeft).unwrap()))
        .unwrap();

    let sample = hello_triangle::d3d11_hello_triangle::Sample::new(
        filter.as_deref().unwrap_or(FILTER_PATH),
        Some(&FilterChainOptionsD3D11 {
            force_no_mipmaps: false,
            disable_cache: false,
            ..Default::default()
        }),
        // replace below with 'None' for the triangle
        Some(image),
    )
    .unwrap();

    // let sample = hello_triangle_old::d3d11_hello_triangle::Sample::new("../test/basic.slangp").unwrap();

    hello_triangle::main(sample).unwrap();
}

/// Example: load an HDR-aware preset, query `color_space()` to see
/// what swapchain color space the host should configure, then construct the
/// filter chain with the matching `hdr_mode`. Non-interactive — no window is
/// opened.
#[test]
fn triangle_d3d11_hdr() {
    use librashader_common::{shader_features::ShaderFeatures, ColorSpace};
    use librashader_presets::{PresetColorSpace, ShaderPreset};
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
    };

    // Match the preset's `#pragma format R16G16B16A16_SFLOAT` last pass: scRGB
    // (linear FP16). The host swapchain must be in the matching color space
    // for the shader output to display correctly.
    let preset =
        ShaderPreset::try_parse("../test/shaders_slang/hdr/hdr.slangp", ShaderFeatures::NONE)
            .unwrap();
    assert_eq!(preset.color_space().unwrap(), ColorSpace::ScRgb);

    // Headless device creation — no swapchain needed.
    let mut device = None;
    let mut feature_level = D3D_FEATURE_LEVEL_11_0;
    let mut context = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut feature_level),
            Some(&mut context),
        )
    }
    .unwrap();
    let device = device.unwrap();

    let _chain = unsafe {
        FilterChainD3D11::load_from_preset(
            preset,
            &device,
            Some(&FilterChainOptionsD3D11 {
                disable_cache: true,
                ..Default::default()
            }),
        )
    }
    .unwrap();
}

#[test]
fn triangle_d3d11() {
    let sample = hello_triangle::d3d11_hello_triangle::Sample::new(
        FILTER_PATH,
        Some(&FilterChainOptionsD3D11 {
            force_no_mipmaps: false,
            disable_cache: false,
            ..Default::default()
        }),
        // replace below with 'None' for the triangle
        None,
        // Some(Image::load(IMAGE_PATH, UVDirection::TopLeft).unwrap()),
    )
    .unwrap();
    // let sample = hello_triangle_old::d3d11_hello_triangle::Sample::new(
    //     "../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp",
    //     Some(&FilterChainOptions {
    //         use_deferred_context: true,
    //     })
    // )
    // .unwrap();

    // let sample = hello_triangle_old::d3d11_hello_triangle::Sample::new("../test/basic.slangp").unwrap();
    println!("{:?}", sample.filter.parameters().parameters());

    hello_triangle::main(sample).unwrap();
}
