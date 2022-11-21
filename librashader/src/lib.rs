
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
    /// Shader compiler targets and runtime for OpenGL.
    pub mod gl {
        /// Shader compiler target for GLSL.
        pub use librashader_reflect::back::targets::GLSL;

        /// Shader runtime for OpenGL.
        pub mod runtime {
            pub use librashader_runtime_gl::*;
        }
    }

    /// Shader compiler targets and runtime for DirectX.
    pub mod dx {
        /// Shader compiler target for HLSL.
        pub use librashader_reflect::back::targets::HLSL;

        /// Shader runtime for DirectX.
        pub mod runtime {

            /// Shader runtime for DirectX 11.
            pub mod dx11 {
                pub use librashader_runtime_dx11::*;
            }

            /// Shader runtime for DirectX 12.
            pub mod dx12 {
                pub use librashader_runtime_dx11::*;
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
    ShaderFormat,
    Size,
    WrapMode
};

