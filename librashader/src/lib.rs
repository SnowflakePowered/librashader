//! Re-exports for usage of librashader in consuming libraries.
//!
//! Runtime implementations should depend directly on constituent crates.


/// Parsing and usage of shader presets.
///
/// Shader presets contain shader and texture parameters, and the order in which to apply a set of shaders
/// in a filter chain.
pub mod presets {
    pub use librashader_presets::*;
    use librashader_preprocess::{PreprocessError, ShaderParameter, ShaderSource};
    use librashader_presets::ShaderPreset;
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

/// Loading and preprocessing of 'slang' shader source files.
///
/// Shader sources files must be loaded with imports resolved before being able to be compiled.
/// Shader parameters are also defined in `#pragma`s within shader source files which must be parsed.
pub mod preprocess {
    pub use librashader_preprocess::*;
}

/// Shader compilation and reflection.
pub mod reflect {
    /// Supported shader compiler targets.
    pub mod targets {
        pub use librashader_reflect::back::targets::GLSL;
        pub use librashader_reflect::back::targets::HLSL;
        pub use librashader_reflect::back::targets::SPIRV;
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
pub mod runtime {
    pub use librashader_runtime::parameters::FilterChainParameters;
    pub use librashader_runtime::filter_chain::FilterChain;

    /// Shader runtime for OpenGL 3.3+.
    pub mod gl {
        pub use librashader_runtime_gl::*;
    }

    /// Shader runtime for Direct3D11
    pub mod d3d11 {
        pub use librashader_runtime_d3d11::*;
    }

    /// Shader compiler targets and runtime for Vulkan.
    pub mod vk {

    }
}

pub use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
