use librashader_preprocess::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::back::{CompileShader, FromCompilation};
use rustc_hash::FxHashMap;
use std::error::Error;
use std::path::Path;
use librashader_reflect::front::shaderc::GlslangCompilation;

use librashader_reflect::reflect::semantics::{ReflectSemantics, SemanticMap, TextureSemantics, UniformSemantic, VariableSemantics};
use librashader_reflect::reflect::ReflectShader;

pub fn load_pass_semantics(
    uniform_semantics: &mut FxHashMap<String, UniformSemantic>,
    texture_semantics: &mut FxHashMap<String, SemanticMap<TextureSemantics>>,
    config: &ShaderPassConfig,
) {
    let Some(alias) = &config.alias else {
        return;
    };

    // Ignore empty aliases
    if alias.trim().is_empty() {
        return;
    }

    let index = config.id as usize;

    // PassOutput
    texture_semantics.insert(
        alias.clone(),
        SemanticMap {
            semantics: TextureSemantics::PassOutput,
            index,
        },
    );
    uniform_semantics.insert(
        format!("{alias}Size"),
        UniformSemantic::Texture(SemanticMap {
            semantics: TextureSemantics::PassOutput,
            index,
        }),
    );

    // PassFeedback
    texture_semantics.insert(
        format!("{alias}Feedback"),
        SemanticMap {
            semantics: TextureSemantics::PassFeedback,
            index,
        },
    );
    uniform_semantics.insert(
        format!("{alias}FeedbackSize"),
        UniformSemantic::Texture(SemanticMap {
            semantics: TextureSemantics::PassFeedback,
            index,
        }),
    );
}

pub fn load(path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let preset = librashader_presets::ShaderPreset::try_parse(path)?;
    let passes: Vec<(&ShaderPassConfig, ShaderSource, _)> = preset
        .shaders
        .iter()
        .map(|shader| {
            let source = ShaderSource::load(&shader.name).unwrap();
            let spirv = GlslangCompilation::compile(&source).unwrap();
            let reflect = HLSL::from_compilation(spirv).unwrap();
            (shader, source, reflect)
        })
        .collect();

    // todo: this can probably be extracted out.
    let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();
    let mut texture_semantics: FxHashMap<String, SemanticMap<TextureSemantics>> =
        Default::default();

    for details in &passes {
        load_pass_semantics(&mut uniform_semantics, &mut texture_semantics, details.0)
    }

    // add float params
    for (_index, parameter) in preset.parameters.iter().enumerate() {
        uniform_semantics.insert(
            parameter.name.clone(),
            UniformSemantic::Variable(SemanticMap {
                semantics: VariableSemantics::FloatParameter,
                index: (),
            }),
        );
    }

    // add lut params
    for (index, texture) in preset.textures.iter().enumerate() {
        texture_semantics.insert(
            texture.name.clone(),
            SemanticMap {
                semantics: TextureSemantics::User,
                index,
            },
        );
    }

    let semantics = ReflectSemantics {
        uniform_semantics,
        non_uniform_semantics: texture_semantics,
    };

    let mut reflections = Vec::new();
    let mut compiled = Vec::new();

    for (index, (_, _, mut reflect)) in passes.into_iter().enumerate() {
        let reflection = reflect.reflect(index, &semantics).unwrap();

        let hlsl = reflect.compile(None).unwrap();

        eprintln!("{:#}", hlsl.vertex);

        eprintln!("{:#}", hlsl.fragment);

        compiled.push(hlsl);
        reflections.push(reflection);
    }

    eprintln!("{:#?}", reflections);

    // //todo: add the semantics for other shit (slang_process:68)
    // eprintln!("{:?}", preset);
    // eprintln!("{:?}", reflect.reflect(&ReflectOptions {
    //     pass_number: i as u32,
    //     uniform_semantics,
    //     non_uniform_semantics: Default::default(),
    // }));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_preset() {
        load("../test/basic.slangp").unwrap();
    }
}
