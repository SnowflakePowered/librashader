#![forbid(missing_docs)]
#![feature(doc_cfg)]
//! RetroArch shader preset compiler and runtime.
//!
//! librashader provides convenient and safe access to RetroArch ['slang' shaders](https://github.com/libretro/slang-shaders).
//! The preset parser, shader preprocessor, and shader runtimes have all been reimplemented in Rust to provide easy access to
//! the rich library of shaders.
//!
//! ## Usage
//! The core objects in librashader are the [`ShaderPreset`](crate::presets::ShaderPreset) and the
//! filter chain implementations.
//!
//! The basic workflow involves parsing a `ShaderPreset`, which can then be used to construct
//! a `FilterChain`. All shaders will then be compiled, after which `FilterChain::frame` can be
//! called with appropriate input and output parameters to draw a frame with the shader effect applied.
//!
//! ## Runtimes
//! Currently available runtimes are Vulkan 1.3+, OpenGL 3.3+ and 4.6 (with DSA), and Direct3D 11.
//! Work on the Direct3D 12 runtime is in progress. The Vulkan runtime requires [`VK_KHR_dynamic_rendering`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_dynamic_rendering.html).
//!
//! | **API**     | **Status** | **`librashader` feature** |
//! |-------------|------------|---------------------------|
//! | OpenGL 3.3+ | ✔         | `gl`                     |
//! | OpenGL 4.6  | ✔         | `gl`                     |
//! | Vulkan      | ✔         | `vk`                     |
//! | Direct3D 11  | ✔         | `d3d11`                  |
//! | Direct3D 12  | 🚧         | `d3d12`                  |
//! | OpenGL 2    | ❌         |                          |
//! | DirectX 9   | ❌         |                          |
//! | Metal       | ❌         |                          |
//!
//! ## C API
//! For documentation on the librashader C API, see [librashader-capi](https://docs.rs/librashader-capi/latest/librashader_capi/),
//! or [`librashader.h`](https://github.com/SnowflakePowered/librashader/blob/master/include/librashader.h).

#[cfg(feature = "presets")]
#[doc(cfg(feature = "presets"))]
/// Parsing and usage of shader presets.
///
/// This module contains facilities and types for parsing `.slangp` shader presets files.
///
/// Shader presets contain shader and texture parameters, and the order in which to apply a set of
/// shaders in a filter chain. A librashader runtime takes a resulting [`ShaderPreset`](crate::presets::ShaderPreset)
/// as input to create a filter chain.
pub mod presets {
    use librashader_preprocess::{PreprocessError, ShaderParameter, ShaderSource};
    pub use librashader_presets::*;
    /// Get full parameter metadata from a shader preset.
    pub fn get_parameter_meta(
        preset: &ShaderPreset,
    ) -> Result<impl Iterator<Item = ShaderParameter>, PreprocessError> {
        let iters: Result<Vec<Vec<ShaderParameter>>, PreprocessError> = preset
            .shaders
            .iter()
            .map(|s| ShaderSource::load(&s.name).map(|s| s.parameters.into_values().collect()))
            .collect();
        let iters = iters?;
        Ok(iters.into_iter().flatten())
    }
}

#[cfg(feature = "preprocess")]
#[doc(cfg(feature = "preprocess"))]
/// Loading and preprocessing of 'slang' shader source files.
///
/// This module contains facilities and types for resolving `#include` directives in `.slang`
/// into a single compilation unit. `#pragma` directives are also parsed and resolved as
/// [`ShaderParameter`](crate::preprocess::ShaderParameter) structs.
///
/// The resulting [`ShaderSource`](crate::preprocess::ShaderSource) can then be passed into a
/// reflection target for reflection and compilation into the target shader format.
pub mod preprocess {
    pub use librashader_preprocess::*;
}

#[cfg(feature = "reflect")]
#[doc(cfg(feature = "reflect"))]
/// Shader reflection and cross-compilation.
///
/// The `type_alias_impl_trait` nightly feature is required. You should choose your
/// target shading language, and a compilation type.
///
/// ```rust
/// #![feature(type_alias_impl_trait)]
///
/// use std::error::Error;
/// use librashader::preprocess::ShaderSource;
/// use librashader::presets::ShaderPreset;
/// use librashader::reflect::{CompileReflectShader, FromCompilation, CompilePresetTarget, ShaderPassArtifact};
/// use librashader::reflect::targets::SPIRV;
/// use librashader::reflect::cross::GlslangCompilation;
/// use librashader::reflect::semantics::ShaderSemantics;
/// type Artifact = impl CompileReflectShader<SPIRV, GlslangCompilation>;
/// type ShaderPassMeta = ShaderPassArtifact<Artifact>;
///
/// // Compile single shader
/// pub fn compile_spirv(
///         source: &ShaderSource,
///     ) -> Result<Artifact, Box<dyn Error>>
/// {
///     let compilation = GlslangCompilation::compile(&source)?;
///     let spirv = SPIRV::from_compilation(artifact)?;
///     Ok(spirv)
/// }
///
/// // Compile preset
/// pub fn compile_preset(preset: ShaderPreset) -> Result<(Vec<ShaderPassMeta>, ShaderSemantics), Box<dyn Error>>
/// {
///     let (passes, semantics) = SPIRV::compile_preset_passes::<GlslangCompilation, Box<dyn Error>>(
///     preset.shaders, &preset.textures)?;
///     Ok((passes, semantics))
/// }
/// ```
///
/// ## What's with all the traits?
/// librashader-reflect is designed to be compiler-agnostic. In the future, we will allow usage of
/// [naga](https://docs.rs/naga/latest/naga/index.html), a pure-Rust shader compiler, when it has
/// matured enough to support [the features librashader needs](https://github.com/gfx-rs/naga/issues/1012).
///
/// In the meanwhile, the only supported compilation type is [GlslangCompilation](crate::reflect::cross::GlslangCompilation),
/// which does transpilation via [shaderc](https://github.com/google/shaderc) and [SPIRV-Cross](https://github.com/KhronosGroup/SPIRV-Cross).
pub mod reflect {
    /// Supported shader compiler targets.
    pub mod targets {
        pub use librashader_reflect::back::targets::GLSL;
        pub use librashader_reflect::back::targets::HLSL;
        pub use librashader_reflect::back::targets::SPIRV;
    }

    pub use librashader_reflect::error::*;

    pub use librashader_reflect::reflect::{semantics, ReflectShader, ShaderReflection};

    pub use librashader_reflect::back::{
        targets::OutputTarget, CompileReflectShader, CompileShader, CompilerBackend,
        FromCompilation, ShaderCompilerOutput,
    };

    /// Reflection via SPIRV-Cross.
    pub mod cross {
        pub use librashader_reflect::front::GlslangCompilation;

        /// The version of GLSL to compile to.
        pub use spirv_cross::glsl::Version as GlslVersion;

        pub use librashader_reflect::back::cross::CrossGlslContext;

        pub use librashader_reflect::back::cross::CrossHlslContext;

        pub use librashader_reflect::reflect::cross::CompiledAst;

        pub use librashader_reflect::reflect::cross::CompiledProgram;
    }
    pub use librashader_reflect::reflect::semantics::BindingMeta;

    pub use librashader_reflect::reflect::presets::{CompilePresetTarget, ShaderPassArtifact};

    pub use librashader_reflect::front::ShaderCompilation;
    #[doc(hidden)]
    #[cfg(feature = "internal")]
    /// Helper methods for runtimes.
    ///
    /// This is internal to librashader runtimes and is exempt from semantic versioning.
    pub mod helper {
        pub use librashader_runtime::image;
    }
}

/// Shader runtimes to execute a filter chain on a GPU surface.
#[cfg(feature = "runtime")]
#[doc(cfg(feature = "runtime"))]
pub mod runtime {
    pub use librashader_common::{Size, Viewport};
    pub use librashader_runtime::parameters::FilterChainParameters;

    #[cfg(feature = "gl")]
    #[doc(cfg(feature = "gl"))]
    /// Shader runtime for OpenGL 3.3+.
    ///
    /// DSA support requires OpenGL 4.6.
    ///
    /// Note that the OpenGL runtime requires `gl` to be
    /// initialized with [`gl::load_with`](https://docs.rs/gl/0.14.0/gl/fn.load_with.html).
    pub mod gl {
        pub use librashader_runtime_gl::{
            error,
            options::{FilterChainOptionsGL as FilterChainOptions, FrameOptionsGL as FrameOptions},
            FilterChainGL as FilterChain, Framebuffer, GLImage,
        };

        #[doc(hidden)]
        #[cfg(feature = "internal")]
        /// Re-exports names to deal with C API conflicts.
        ///
        /// This is internal to librashader-capi and is exempt from semantic versioning.
        pub mod capi {
            pub use librashader_runtime_gl::*;
        }
    }

    #[cfg(all(target_os = "windows", feature = "d3d11"))]
    #[doc(cfg(all(target_os = "windows", feature = "d3d11")))]
    /// Shader runtime for Direct3D 11.
    pub mod d3d11 {
        pub use librashader_runtime_d3d11::{
            error,
            options::{
                FilterChainOptionsD3D11 as FilterChainOptions, FrameOptionsD3D11 as FrameOptions,
            },
            D3D11InputView, D3D11OutputView, FilterChainD3D11 as FilterChain,
        };

        #[doc(hidden)]
        #[cfg(feature = "internal")]
        /// Re-exports names to deal with C API conflicts.
        ///
        /// This is internal to librashader-capi and is exempt from semantic versioning.
        pub mod capi {
            pub use librashader_runtime_d3d11::*;
        }
    }

    #[cfg(feature = "vk")]
    #[doc(cfg(feature = "vk"))]
    /// Shader runtime for Vulkan 1.3+.
    pub mod vk {
        pub use librashader_runtime_vk::{
            error,
            options::{
                FilterChainOptionsVulkan as FilterChainOptions, FrameOptionsVulkan as FrameOptions,
            },
            FilterChainVulkan as FilterChain, VulkanImage, VulkanInstance, VulkanObjects,
        };

        #[doc(hidden)]
        #[cfg(feature = "internal")]
        /// Re-exports names to deal with C API conflicts.
        ///
        /// This is internal to librashader-capi and is exempt from semantic versioning.
        pub mod capi {
            pub use librashader_runtime_vk::*;
        }
    }
}

pub use librashader_common::{FilterMode, ImageFormat, WrapMode};
