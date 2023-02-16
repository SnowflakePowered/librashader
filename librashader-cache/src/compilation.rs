//!  Cache helpers for `ShaderCompilation` objects to cache compiled SPIRV.
use librashader_preprocess::ShaderSource;
use librashader_reflect::back::targets::{DXIL, GLSL, HLSL, SPIRV};
use librashader_reflect::back::{CompilerBackend, FromCompilation};
use librashader_reflect::error::{ShaderCompileError, ShaderReflectError};
use librashader_reflect::front::{GlslangCompilation, ShaderCompilation};

pub struct CachedCompilation<T> {
    compilation: T,
}

impl<T: ShaderCompilation + for<'de> serde::Deserialize<'de> + serde::Serialize + Clone>
    ShaderCompilation for CachedCompilation<T>
{
    fn compile(source: &ShaderSource) -> Result<Self, ShaderCompileError> {
        let cache = crate::cache::internal::get_cache();

        let Ok(cache) = cache else {
            return Ok(CachedCompilation {
                compilation: T::compile(source)?
            })
        };

        let key = {
            let mut hasher = blake3::Hasher::new();
            hasher.update(source.vertex.as_bytes());
            hasher.update(source.fragment.as_bytes());
            let hash = hasher.finalize();
            hash
        };

        let compilation = 'cached: {
            if let Ok(cached) = crate::cache::internal::get_blob(&cache, "spirv", key.as_bytes()) {
                let decoded =
                    bincode::serde::decode_from_slice(&cached, bincode::config::standard())
                        .map(|(compilation, _)| CachedCompilation { compilation })
                        .ok();

                if let Some(compilation) = decoded {
                    break 'cached compilation;
                }
            }

            CachedCompilation {
                compilation: T::compile(source)?,
            }
        };

        if let Ok(updated) =
            bincode::serde::encode_to_vec(&compilation.compilation, bincode::config::standard())
        {
            crate::cache::internal::set_blob(&cache, "spirv", key.as_bytes(), &updated)
        }

        Ok(compilation)
    }
}

impl FromCompilation<CachedCompilation<GlslangCompilation>> for DXIL {
    type Target = <DXIL as FromCompilation<GlslangCompilation>>::Target;
    type Options = <DXIL as FromCompilation<GlslangCompilation>>::Options;
    type Context = <DXIL as FromCompilation<GlslangCompilation>>::Context;
    type Output = <DXIL as FromCompilation<GlslangCompilation>>::Output;

    fn from_compilation(
        compile: CachedCompilation<GlslangCompilation>,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        DXIL::from_compilation(compile.compilation)
    }
}

impl FromCompilation<CachedCompilation<GlslangCompilation>> for HLSL {
    type Target = <HLSL as FromCompilation<GlslangCompilation>>::Target;
    type Options = <HLSL as FromCompilation<GlslangCompilation>>::Options;
    type Context = <HLSL as FromCompilation<GlslangCompilation>>::Context;
    type Output = <HLSL as FromCompilation<GlslangCompilation>>::Output;

    fn from_compilation(
        compile: CachedCompilation<GlslangCompilation>,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        HLSL::from_compilation(compile.compilation)
    }
}

impl FromCompilation<CachedCompilation<GlslangCompilation>> for GLSL {
    type Target = <GLSL as FromCompilation<GlslangCompilation>>::Target;
    type Options = <GLSL as FromCompilation<GlslangCompilation>>::Options;
    type Context = <GLSL as FromCompilation<GlslangCompilation>>::Context;
    type Output = <GLSL as FromCompilation<GlslangCompilation>>::Output;

    fn from_compilation(
        compile: CachedCompilation<GlslangCompilation>,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        GLSL::from_compilation(compile.compilation)
    }
}

impl FromCompilation<CachedCompilation<GlslangCompilation>> for SPIRV {
    type Target = <SPIRV as FromCompilation<GlslangCompilation>>::Target;
    type Options = <SPIRV as FromCompilation<GlslangCompilation>>::Options;
    type Context = <SPIRV as FromCompilation<GlslangCompilation>>::Context;
    type Output = <SPIRV as FromCompilation<GlslangCompilation>>::Output;

    fn from_compilation(
        compile: CachedCompilation<GlslangCompilation>,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        SPIRV::from_compilation(compile.compilation)
    }
}
