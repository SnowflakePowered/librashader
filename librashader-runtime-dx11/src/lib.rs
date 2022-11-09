use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use rustc_hash::FxHashMap;
use librashader::ShaderSource;
use librashader_presets::ShaderPassConfig;
use librashader_reflect::back::ShaderCompiler;
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::front::shaderc::GlslangCompilation;
use librashader_reflect::reflect::cross::{CrossReflect, HlslOptions, HlslReflect};
use librashader_reflect::reflect::{ReflectSemantics, ReflectShader, UniformSemantic};
use librashader_reflect::reflect::semantics::{SemanticMap, TextureSemantics, VariableSemantics};


pub fn load_pass_semantics(uniform_semantics: &mut FxHashMap<String, UniformSemantic>, texture_semantics: &mut FxHashMap<String, SemanticMap<TextureSemantics>>,
                           config: &ShaderPassConfig) {
    let Some(alias) = &config.alias else {
        return;
    };

    // Ignore empty aliases
    if alias.trim().is_empty() {
        return;
    }

    let index = config.id as u32;

    // PassOutput
    texture_semantics.insert(alias.clone(), SemanticMap {
        semantics: TextureSemantics::PassOutput,
        index
    });
    uniform_semantics.insert(format!("{alias}Size"), UniformSemantic::Texture(SemanticMap {
        semantics: TextureSemantics::PassOutput,
        index
    }));

    // PassFeedback
    texture_semantics.insert(format!("{alias}Feedback"), SemanticMap {
        semantics: TextureSemantics::PassFeedback,
        index
    });
    uniform_semantics.insert(format!("{alias}FeedbackSize"), UniformSemantic::Texture(SemanticMap {
        semantics: TextureSemantics::PassFeedback,
        index
    }));

}

pub fn load(path: impl AsRef<Path>) -> Result<(), Box<dyn Error>>{
    let preset = librashader_presets::ShaderPreset::try_parse(path)?;
    let mut passes: Vec<(&ShaderPassConfig, ShaderSource, HLSL)> = preset.shaders.iter()
        .map(|shader| {
            let source = librashader_preprocess::load_shader_source(&shader.name)
                .unwrap();
            let spirv = librashader_reflect::front::shaderc::compile_spirv(&source)
                .unwrap();
            let mut reflect = librashader_reflect::back::targets::HLSL::try_from(spirv).unwrap();
            (shader, source, reflect)
        }).collect();

    // todo: this can probably be extracted out.
    let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();
    let mut texture_semantics: FxHashMap<String, SemanticMap<TextureSemantics>> = Default::default();

    for details in &passes {
        load_pass_semantics(&mut uniform_semantics, &mut texture_semantics, details.0)
    }

    // add float params
    for (index, parameter) in preset.parameters.iter().enumerate() {
        uniform_semantics.insert(parameter.name.clone(), UniformSemantic::Variable(SemanticMap {
            semantics: VariableSemantics::FloatParameter,
            index: index as u32
        }));
    }

    // add lut params
    for (index, texture) in preset.textures.iter().enumerate() {
        texture_semantics.insert(texture.name.clone(), SemanticMap {
            semantics: TextureSemantics::User,
            index: index as u32
        });
    }

    let semantics = ReflectSemantics {
        uniform_semantics,
        non_uniform_semantics: texture_semantics
    };

    let mut reflections = Vec::new();
    let mut compiled = Vec::new();

    for (index, (_, _, reflect)) in passes.iter_mut().enumerate() {
        let reflection = reflect.reflect(index as u32, &semantics)
            .unwrap();

        let hlsl = reflect.compile(None)
            .unwrap();

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

        load("../test/basic.slangp")
            .unwrap();
    }
}
