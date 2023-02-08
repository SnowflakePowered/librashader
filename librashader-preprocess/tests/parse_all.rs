use glob::glob;
use librashader_preprocess::{PreprocessError, ShaderSource};
use librashader_presets::ShaderPreset;
#[test]
fn preprocess_all_slang_presets_parsed() {
    for entry in glob("../test/slang-shaders/**/*.slangp").unwrap() {
        if let Ok(path) = entry {
            if let Ok(preset) = ShaderPreset::try_parse(&path) {
                for shader in preset.shaders {
                    ShaderSource::load(&shader.name).expect(&format!(
                        "Failed to preprocess shader {} from preset {}",
                        shader.name.display(),
                        path.display()
                    ));
                }
            }
        }
    }
}
