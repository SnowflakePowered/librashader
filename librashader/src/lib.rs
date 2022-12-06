#![forbid(missing_docs)]
//! RetroArch shader preset compiler and runtime.
//!
//! librashader provides convenient and safe access to RetroArch ['slang' shaders](https://github.com/libretro/slang-shaders).
//! The preset parser, shader preprocessor, and shader runtimes have all been reimplemented in Rust to provide easy access to
//! the rich library of shaders.
//!
//! ## Usage
//! The core objects in librashader are the [`ShaderPreset`](crate::presets::ShaderPreset) and the
//! [`FilterChain`](crate::runtime::FilterChain), the implementation of which is runtime dependent.
//!
//! The basic workflow involves parsing a `ShaderPreset`, which can then be used to construct
//! a `FilterChain`. All shaders will then be compiled, after which `FilterChain::frame` can be
//! called with appropriate input and output parameters to draw a frame with the shader effect applied.
//!
//! ## Runtimes
//! Currently available runtimes are OpenGL 3.3+ and 4.6 (with DSA), and Direct3D 11.
//! Work on the Vulkan and Direct3D 12 runtimes are in progress.
//!
//! | **API**     | **Status** | **`librashader` feature** |
//! |-------------|------------|---------------------------|
//! | OpenGL 3.3+ | ✔         | `gl`                     |
//! | OpenGL 4.6  | ✔         | `gl`                     |
//! | Vulkan      | 🚧         | `vk`                     |
//! | Direct3D 11  | ✔         | `d3d11`                  |
//! | Direct3D 12  | 🚧         | `d3d12`                  |
//! | OpenGL 2    | ❌         |                          |
//! | DirectX 9   | ❌         |                          |
//! | Metal       | ❌         |                          |
//!

#[cfg(feature = "presets")]
/// Parsing and usage of shader presets.
///
/// Shader presets contain shader and texture parameters, and the order in which to apply a set of shaders
/// in a filter chain.
pub mod presets {
    pub use librashader_presets::*;
    use librashader_preprocess::{PreprocessError, ShaderParameter, ShaderSource};
    /// Get full parameter metadata from a shader preset.
    pub fn get_parameter_meta(
        preset: &ShaderPreset,
    ) -> Result<impl Iterator<Item = ShaderParameter>, PreprocessError> {
        let iters: Result<Vec<Vec<ShaderParameter>>, PreprocessError> = preset
            .shaders
            .iter()
            .map(|s| ShaderSource::load(&s.name).map(|s| s.parameters))
            .into_iter()
            .collect();
        let iters = iters?;
        Ok(iters.into_iter().flatten())
    }
}

#[cfg(feature = "preprocess")]
/// Loading and preprocessing of 'slang' shader source files.
///
/// Shader sources files must be loaded with imports resolved before being able to be compiled.
/// Shader parameters are also defined in `#pragma`s within shader source files which must be parsed.
pub mod preprocess {
    pub use librashader_preprocess::*;
}

#[cfg(feature = "reflect")]
/// Shader compilation and reflection.
pub mod reflect {
    /// Supported shader compiler targets.
    pub mod targets {
        pub use librashader_reflect::back::targets::GLSL;
        pub use librashader_reflect::back::targets::HLSL;
        pub use librashader_reflect::back::targets::SpirV;
    }

    pub use librashader_reflect::error::*;

    pub use librashader_reflect::reflect::{
        ReflectShader, semantics, ShaderReflection,
    };

    pub use librashader_reflect::back::{
        CompilerBackend, CompileShader, FromCompilation, ShaderCompilerOutput,
        targets::OutputTarget,
    };
    pub use librashader_reflect::front::shaderc::GlslangCompilation;
    pub use librashader_reflect::reflect::semantics::BindingMeta;
}

/// Shader runtimes to execute a filter chain on a GPU surface.
#[cfg(feature = "runtime")]
pub mod runtime {
    pub use librashader_runtime::parameters::FilterChainParameters;
    pub use librashader_runtime::filter_chain::FilterChain;

    #[cfg(feature = "gl")]
    /// Shader runtime for OpenGL 3.3+.
    ///
    /// Note that the OpenGL runtime requires `gl` to be
    /// initialized with [`gl::load_with`](https://docs.rs/gl/0.14.0/gl/fn.load_with.html).
    pub mod gl {
        pub use librashader_runtime_gl::*;
    }

    #[cfg(feature = "d3d11")]
    /// Shader runtime for Direct3D11
    pub mod d3d11 {
        pub use librashader_runtime_d3d11::*;
    }

    #[cfg(feature = "vk")]
    /// Shader compiler targets and runtime for Vulkan.
    pub mod vk {

    }
}

pub use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
