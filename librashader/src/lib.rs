//! Re-exports for usage of librashader in consuming libraries.
//!
//! Runtime implementations should depend directly on constituent crates.
pub mod presets {
    pub use librashader_presets::*;
}

pub mod preprocess {
    pub use librashader_preprocess::*;
}

pub mod reflect {
    pub use librashader_reflect::error::*;

    pub use librashader_reflect::reflect::{
        ReflectMeta, ReflectShader, semantics, ShaderReflection
    };

    pub use librashader_reflect::front::shaderc::GlslangCompilation;
    pub use librashader_reflect::back::{
        CompileShader,
        FromCompilation,
        ShaderCompilerOutput,
        CompilerBackend,
        targets::OutputTarget,
    };
}

pub mod targets {
    /// Shader compiler targets and runtime for OpenGL 3.3+.
    pub mod gl {
        /// Shader compiler target for GLSL.
        pub use librashader_reflect::back::targets::GLSL;

        /// Shader runtime for OpenGL.
        pub mod runtime {
            pub use librashader_runtime_gl::*;
        }
    }

    /// Shader compiler targets and runtime for OpenGL 4.6.
    pub mod gl46 {
        /// Shader compiler target for GLSL.
        pub use librashader_reflect::back::targets::GLSL;

        /// Shader runtime for OpenGL.
        pub mod runtime {
            pub use librashader_runtime_gl46::*;
        }
    }

    /// Shader compiler targets and runtime for DirectX.
    pub mod dx {
        /// Shader compiler target for HLSL.
        pub use librashader_reflect::back::targets::HLSL;

        /// Shader runtime for DirectX.
        pub mod runtime {

            /// Shader runtime for DirectX 11.
            pub mod d3d11 {
                pub use librashader_runtime_d3d11::*;
            }
        }
    }

    /// Shader compiler targets and runtime for Vulkan.
    pub mod vk {
        /// Shader compiler target for SPIR-V.
        pub use librashader_reflect::back::targets::SPIRV;
    }
}

pub use librashader_common::{
    FilterMode,
    ImageFormat,
    Size,
    WrapMode
};

pub mod util {
    use librashader_preprocess::{PreprocessError, ShaderParameter, ShaderSource};
    use librashader_presets::ShaderPreset;

    pub fn get_parameter_meta(preset: &ShaderPreset) -> Result<impl Iterator<Item = ShaderParameter>, PreprocessError> {
        let iters: Result<Vec<Vec<ShaderParameter>>, PreprocessError> = preset.shaders.iter()
            .map(|s| ShaderSource::load(&s.name).map(|s| s.parameters))
            .into_iter()
            .collect();
        let iters = iters?;
        Ok(iters.into_iter().flatten())
    }
}
