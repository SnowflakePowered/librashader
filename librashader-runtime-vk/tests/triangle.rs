mod hello_triangle;

use hello_triangle::vulkan_base::VulkanBase;
use librashader_common::shader_features::ShaderFeatures;
use librashader_runtime_vk::options::FilterChainOptionsVulkan;
use librashader_runtime_vk::FilterChainVulkan;

/// Example: load an HDR-aware preset, query `color_space()` to see
/// what swapchain color space the host should configure, then construct the
/// filter chain with the matching `hdr_mode`. Non-interactive — no window is
/// opened.
#[test]
fn triangle_vk_hdr() {
    use librashader_common::ColorSpace;
    use librashader_presets::{PresetColorSpace, ShaderPreset};

    let hdr_mode = ColorSpace::ScRgb;
    let preset =
        ShaderPreset::try_parse("../test/shaders_slang/hdr/hdr.slangp", ShaderFeatures::NONE)
            .unwrap();
    assert_eq!(
        preset.color_space().unwrap(),
        ColorSpace::ScRgb
    );

    let entry = unsafe { ash::Entry::load().unwrap() };
    let base = VulkanBase::new(entry).unwrap();

    let _chain = unsafe {
        FilterChainVulkan::load_from_preset(
            preset,
            &base,
            Some(&FilterChainOptionsVulkan {
                hdr_mode,
                disable_cache: true,
                ..Default::default()
            }),
        )
    }
    .unwrap();
}

#[test]
fn triangle_vk() {
    let entry = unsafe { ash::Entry::load().unwrap() };
    let base = VulkanBase::new(entry).unwrap();

    unsafe {
        let filter = FilterChainVulkan::load_from_path(
            "../test/shaders_slang/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp",
            // "../test/shaders_slang/sonkun/slot-mask/curved-screen/1080p/01-1080p-crt-guest-advanced-hd-slot-mask-u-normal-rf.slangp",
            ShaderFeatures::all(),
            // "../test/Mega_Bezel_Packs/Duimon-Mega-Bezel/Presets/Advanced/Nintendo_GBA_SP/GBA_SP-[ADV]-[LCD-GRID]-[Night].slangp",
            &base,
            // "../test/basic.slangp",
            Some(&FilterChainOptionsVulkan {
                frames_in_flight: 3,
                force_no_mipmaps: false,
                use_dynamic_rendering: false,
                disable_cache: true,
                ..Default::default()
            }),
        )
        .unwrap();

        hello_triangle::main(base, filter)
    }

    // let base = hello_triangle_old::ExampleBase::new(900, 600);
    // // let mut filter = FilterChainVulkan::load_from_path(
    // //     (base.device.clone(), base.present_queue.clone(), base.device_memory_properties.clone()),
    // //     "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
    // //     None
    // // )
    //
    // let mut filter = FilterChainVulkan::load_from_path(
    //     (
    //         base.device.clone(),
    //         base.present_queue.clone(),
    //         base.device_memory_properties.clone(),
    //     ),
    //     "../test/slang-shaders/border/gameboy-player/gameboy-player-crt-royale.slangp",
    //     None,
    // )
    // // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
    // .unwrap();
    // hello_triangle_old::main(base, filter);
}
