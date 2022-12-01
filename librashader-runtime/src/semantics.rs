use librashader_presets::{ShaderPassConfig, TextureConfig};
use librashader_reflect::reflect::semantics::{SemanticMap, TextureSemantics, UniformSemantic};
use rustc_hash::FxHashMap;

/// A map for variable names and uniform semantics
pub type UniformSemanticsMap = FxHashMap<String, UniformSemantic>;

/// A map for sampler names and texture semantics.
pub type TextureSemanticsMap = FxHashMap<String, SemanticMap<TextureSemantics>>;

/// Insert the available semantics for the input pass config into the provided semantic maps.
pub fn insert_pass_semantics(
    uniform_semantics: &mut UniformSemanticsMap,
    texture_semantics: &mut TextureSemanticsMap,
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

/// /// Insert the available semantics for the input texture config into the provided semantic maps.
pub fn insert_lut_semantics(
    textures: &[TextureConfig],
    uniform_semantics: &mut UniformSemanticsMap,
    texture_semantics: &mut TextureSemanticsMap,
) {
    for (index, texture) in textures.iter().enumerate() {
        texture_semantics.insert(
            texture.name.clone(),
            SemanticMap {
                semantics: TextureSemantics::User,
                index,
            },
        );

        uniform_semantics.insert(
            format!("{}Size", texture.name),
            UniformSemantic::Texture(SemanticMap {
                semantics: TextureSemantics::User,
                index,
            }),
        );
    }
}
