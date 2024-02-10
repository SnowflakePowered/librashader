//!  Cache helpers for `ShaderCompilation` objects to cache compiled SPIRV.
use librashader_preprocess::ShaderSource;
#[cfg(all(target_os = "windows", feature = "d3d"))]
use librashader_reflect::back::targets::DXIL;
use librashader_reflect::back::targets::{GLSL, HLSL, SPIRV};

use librashader_reflect::back::{CompilerBackend, FromCompilation};
use librashader_reflect::error::{ShaderCompileError, ShaderReflectError};
use librashader_reflect::front::{
    Glslang, ShaderInputCompiler, ShaderReflectObject, SpirvCompilation,
};

pub struct CachedCompilation<T> {
    compilation: T,
}

impl<T: ShaderReflectObject> ShaderReflectObject for CachedCompilation<T> {}

impl<T: ShaderReflectObject + for<'de> serde::Deserialize<'de> + serde::Serialize + Clone>
    ShaderInputCompiler<CachedCompilation<T>> for Glslang
where
    Glslang: ShaderInputCompiler<T>,
{
    fn compile(source: &ShaderSource) -> Result<CachedCompilation<T>, ShaderCompileError> {
        let cache = crate::cache::internal::get_cache();

        let Ok(cache) = cache else {
            return Ok(CachedCompilation {
                compilation: Glslang::compile(source)?,
            });
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
                compilation: Glslang::compile(source)?,
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

#[cfg(all(target_os = "windows", feature = "d3d"))]
impl FromCompilation<CachedCompilation<SpirvCompilation>> for DXIL {
    type Target = <DXIL as FromCompilation<SpirvCompilation>>::Target;
    type Options = <DXIL as FromCompilation<SpirvCompilation>>::Options;
    type Context = <DXIL as FromCompilation<SpirvCompilation>>::Context;
    type Output = <DXIL as FromCompilation<SpirvCompilation>>::Output;

    fn from_compilation(
        compile: CachedCompilation<SpirvCompilation>,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        DXIL::from_compilation(compile.compilation)
    }
}

impl FromCompilation<CachedCompilation<SpirvCompilation>> for HLSL {
    type Target = <HLSL as FromCompilation<SpirvCompilation>>::Target;
    type Options = <HLSL as FromCompilation<SpirvCompilation>>::Options;
    type Context = <HLSL as FromCompilation<SpirvCompilation>>::Context;
    type Output = <HLSL as FromCompilation<SpirvCompilation>>::Output;

    fn from_compilation(
        compile: CachedCompilation<SpirvCompilation>,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        HLSL::from_compilation(compile.compilation)
    }
}

impl FromCompilation<CachedCompilation<SpirvCompilation>> for GLSL {
    type Target = <GLSL as FromCompilation<SpirvCompilation>>::Target;
    type Options = <GLSL as FromCompilation<SpirvCompilation>>::Options;
    type Context = <GLSL as FromCompilation<SpirvCompilation>>::Context;
    type Output = <GLSL as FromCompilation<SpirvCompilation>>::Output;

    fn from_compilation(
        compile: CachedCompilation<SpirvCompilation>,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        GLSL::from_compilation(compile.compilation)
    }
}

impl FromCompilation<CachedCompilation<SpirvCompilation>> for SPIRV {
    type Target = <SPIRV as FromCompilation<SpirvCompilation>>::Target;
    type Options = <SPIRV as FromCompilation<SpirvCompilation>>::Options;
    type Context = <SPIRV as FromCompilation<SpirvCompilation>>::Context;
    type Output = <SPIRV as FromCompilation<SpirvCompilation>>::Output;

    fn from_compilation(
        compile: CachedCompilation<SpirvCompilation>,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        SPIRV::from_compilation(compile.compilation)
    }
}
