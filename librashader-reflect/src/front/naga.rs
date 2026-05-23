use crate::error::ShaderCompileError;
use librashader_common::map::ShortString;
use librashader_preprocess::ShaderSource;

use crate::front::{ShaderInputCompiler, ShaderReflectObject, WgslCompilation};
use crate::reflect::semantics::ShaderSemantics;

/// glslang compiler
pub struct NagaWgsl;

impl ShaderReflectObject for WgslCompilation {
    type Compiler = NagaWgsl;
}

impl TryFrom<&ShaderSource> for WgslCompilation {
    type Error = ShaderCompileError;

    /// Tries to compile  from the provided shader source.
    fn try_from(source: &ShaderSource) -> Result<Self, Self::Error> {
        NagaWgsl::compile(source)
    }
}

impl ShaderInputCompiler<WgslCompilation> for NagaWgsl {
    fn compile(source: &ShaderSource) -> Result<WgslCompilation, ShaderCompileError> {
        parse_wgsl(source)
    }

    fn apply_mangled_semantics(semantics: &mut ShaderSemantics) {
        let mut namer = naga::proc::Namer::default();
        let mut sink = naga::FastHashMap::default();
        namer.reset(
            &naga::Module::default(),
            &naga::keywords::wgsl::RESERVED_SET,
            &naga::keywords::wgsl::BUILTIN_IDENTIFIER_SET,
            &naga::proc::CaseInsensitiveKeywordSet::empty(),
            &["__", "_naga"],
            &mut sink,
        );

        let uniform_aliases: Vec<(ShortString, _)> = semantics
            .uniform_semantics
            .iter()
            .filter_map(|(key, value)| {
                let mangled = namer.call(key.as_ref());
                if mangled != key.as_ref() {
                    Some((ShortString::from(mangled.as_str()), value.clone()))
                } else {
                    None
                }
            })
            .collect();
        for (key, value) in uniform_aliases {
            semantics.uniform_semantics.entry(key).or_insert(value);
        }

        let texture_aliases: Vec<(ShortString, _)> = semantics
            .texture_semantics
            .iter()
            .filter_map(|(key, value)| {
                let mangled = namer.call(key.as_ref());
                if mangled != key.as_ref() {
                    Some((ShortString::from(mangled.as_str()), value.clone()))
                } else {
                    None
                }
            })
            .collect();
        for (key, value) in texture_aliases {
            semantics.texture_semantics.entry(key).or_insert(value);
        }
    }
}

pub(crate) fn parse_wgsl(source: &ShaderSource) -> Result<WgslCompilation, ShaderCompileError> {
    let vertex: naga::Module = naga::front::wgsl::parse_str(&source.vertex)?;
    let fragment: naga::Module = naga::front::wgsl::parse_str(&source.fragment)?;

    Ok(WgslCompilation { vertex, fragment })
}
