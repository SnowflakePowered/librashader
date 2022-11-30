use librashader_presets::{ShaderPassConfig, TextureConfig};
use librashader_reflect::reflect::semantics::{SemanticMap, TextureSemantics, UniformSemantic};
use rustc_hash::FxHashMap;

pub type UniformSemanticsMap = FxHashMap<String, UniformSemantic>;
pub type TextureSemanticsMap = FxHashMap<String, SemanticMap<TextureSemantics>>;

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
