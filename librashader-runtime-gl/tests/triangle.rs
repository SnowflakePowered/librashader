mod hello_triangle;

use librashader_common::shader_features::ShaderFeatures;
use librashader_runtime_gl::options::FilterChainOptionsGL;
use librashader_runtime_gl::FilterChainGL;
use std::sync::Arc;

/// Example: load an HDR-aware preset, query `color_space()` to see
/// what swapchain color space the host should configure, then construct the
/// filter chain with the matching `hdr_mode`.
///
/// Briefly opens a window because GLFW requires a context — closes immediately
/// after the assertions.
#[test]
fn triangle_gl_hdr() {
    use glfw::{fail_on_errors, Context};
    use librashader_common::ColorSpace;
    use librashader_presets::{PresetColorSpace, ShaderPreset};

    let color_space = ColorSpace::ScRgb;
    let preset =
        ShaderPreset::try_parse("../test/shaders_slang/hdr/hdr.slangp", ShaderFeatures::NONE)
            .unwrap();
    assert_eq!(preset.color_space().unwrap(), ColorSpace::ScRgb);

    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
    glfw.window_hint(glfw::WindowHint::Visible(false));

    let (mut window, _events) = glfw
        .create_window(1, 1, "hdr-test", glfw::WindowMode::Windowed)
        .unwrap();
    window.make_current();
    let context = Arc::new(unsafe {
        glow::Context::from_loader_function(|ptr| window.get_proc_address(ptr))
    });

    let _filter = unsafe {
        FilterChainGL::load_from_preset(
            preset,
            Arc::clone(&context),
            Some(&FilterChainOptionsGL {
                glsl_version: 330,
                use_dsa: false,
                force_no_mipmaps: false,
                disable_cache: true,
                color_space,
            }),
        )
    }
    .unwrap();
}

#[test]
fn triangle_gl() {
    let (glfw, window, events, shader, vao, context) = hello_triangle::gl3::setup();

    unsafe {
        let mut filter = FilterChainGL::load_from_path(
            // "../test/basic.slangp",
            "../test/shaders_slang/crt/crt-royale.slangp",
            ShaderFeatures::NONE,
            Arc::clone(&context),
            Some(&FilterChainOptionsGL {
                glsl_version: 0,
                use_dsa: false,
                force_no_mipmaps: false,
                disable_cache: true,
                ..Default::default()
            }),
        )
        // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
        .expect("Failed to load filter chain");
        hello_triangle::gl3::do_loop(&context, glfw, window, events, shader, vao, &mut filter);
    }
}

#[test]
fn triangle_gl46() {
    let (glfw, window, events, shader, vao, context) = hello_triangle::gl46::setup();
    unsafe {
        let mut filter = FilterChainGL::load_from_path(
            // "../test/slang-shaders/vhs/VHSPro.slangp",
            // "../test/slang-shaders/test/history.slangp",
            // "../test/basic.slangp",
            // "../test/shaders_slang/crt/crt-royale.slangp",
            // "../test/shaders_slang/crt/crt-royale.slangp",
            "../test/shaders_slang/sonkun/slot-mask/flat-screen/1080p/test.slangp",
            ShaderFeatures::NONE,
            Arc::clone(&context),
            Some(&FilterChainOptionsGL {
                glsl_version: 330,
                use_dsa: true,
                force_no_mipmaps: false,
                disable_cache: false,
                ..Default::default()
            }),
        )
        // FilterChain::load_from_path("../test/slang-shaders/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp", None)
        .unwrap();
        hello_triangle::gl46::do_loop(&context, glfw, window, events, shader, vao, &mut filter);
    }
}
