use librashader_preprocess::{PreprocessError, ShaderSource};
use librashader_presets::{ShaderPassConfig, TextureConfig};
use librashader_reflect::back::targets::OutputTarget;
use librashader_reflect::back::{CompilerBackend, FromCompilation};
use librashader_reflect::error::{ShaderCompileError, ShaderReflectError};
use librashader_reflect::front::ShaderCompilation;
use librashader_reflect::reflect::semantics::{
    Semantic, ShaderSemantics, TextureSemantics, UniformSemantic, UniqueSemantics,
};
use rustc_hash::FxHashMap;
use std::error::Error;

pub type ShaderPassMeta<T> = (ShaderPassConfig, ShaderSource, CompilerBackend<T>);

/// Compile passes of a shader preset given the applicable
/// shader output target, compilation type, and resulting error.
pub fn compile_preset_passes<T, C, E>(
    passes: Vec<ShaderPassConfig>,
    textures: &[TextureConfig],
) -> Result<
    (
        Vec<ShaderPassMeta<<T as FromCompilation<C>>::Output>>,
        ShaderSemantics,
    ),
    E,
>
where
    T: OutputTarget,
    T: FromCompilation<C>,
    C: ShaderCompilation,
    E: Error,
    E: From<PreprocessError>,
    E: From<ShaderReflectError>,
    E: From<ShaderCompileError>,
{
    let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();
    let mut texture_semantics: FxHashMap<String, Semantic<TextureSemantics>> = Default::default();

    let passes = passes
        .into_iter()
        .map(|shader| {
            let source: ShaderSource = ShaderSource::load(&shader.name)?;

            let compiled = C::compile(&source)?;
            let reflect = T::from_compilation(compiled)?;

            for parameter in source.parameters.values() {
                uniform_semantics.insert(
                    parameter.id.clone(),
                    UniformSemantic::Unique(Semantic {
                        semantics: UniqueSemantics::FloatParameter,
                        index: (),
                    }),
                );
            }
            Ok::<_, E>((shader, source, reflect))
        })
        .collect::<Result<Vec<(ShaderPassConfig, ShaderSource, CompilerBackend<_>)>, E>>()?;

    for details in &passes {
        crate::semantics::insert_pass_semantics(
            &mut uniform_semantics,
            &mut texture_semantics,
            &details.0,
        )
    }
    crate::semantics::insert_lut_semantics(
        textures,
        &mut uniform_semantics,
        &mut texture_semantics,
    );

    let semantics = ShaderSemantics {
        uniform_semantics,
        texture_semantics,
    };

    Ok((passes, semantics))
}
