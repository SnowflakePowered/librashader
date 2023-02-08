use glob::glob;
use librashader_presets::ShaderPreset;

#[test]
fn parses_all_slang_presets() {
    for entry in glob("../test/slang-shaders/**/*.slangp").unwrap() {
        if let Ok(path) = entry {
            if let Err(e) = ShaderPreset::try_parse(&path) {
                println!("Could not parse {}: {:?}", path.display(), e)
            }
        }
    }
}

#[test]
fn parses_problematic() {
    for entry in glob("../test/slang-shaders/crt/crt-hyllian-sinc-glow.slangp").unwrap() {
        if let Ok(path) = entry {
            ShaderPreset::try_parse(&path).expect(&format!("Failed to parse {}", path.display()));
        }
    }
}
