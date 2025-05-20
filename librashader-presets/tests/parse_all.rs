use glob::glob;
use librashader_common::shader_features::ShaderFeatures;
use librashader_presets::context::{ContextItem, VideoDriver, WildcardContext};
use librashader_presets::ShaderPreset;

#[test]
fn parses_all_slang_presets() {
    for entry in glob("../test/shaders_slang/**/*.slangp").unwrap() {
        if let Ok(path) = entry {
            if let Err(e) = ShaderPreset::try_parse(&path, ShaderFeatures::empty()) {
                println!("Could not parse {}: {:?}", path.display(), e)
            }
        }
    }
}

#[test]
fn parses_problematic() {
    let path  = "../test/Mega_Bezel_Packs/Duimon-Mega-Bezel/Presets/Advanced/Nintendo_NDS_DREZ/NDS-[DREZ]-[Native]-[ADV]-[Guest]-[Night].slangp";
    ShaderPreset::try_parse(path, ShaderFeatures::empty())
        .expect(&format!("Failed to parse {}", path));
}

#[test]
fn parses_wildcard() {
    let path =
        "../test/shaders_slang/bezel/Mega_Bezel/resource/wildcard-examples/Preset-01-Core.slangp";
    let mut context = WildcardContext::new();

    context.add_video_driver_defaults(VideoDriver::Vulkan);

    context.append_item(ContextItem::CoreName(String::from("image display")));

    ShaderPreset::try_parse_with_context(path, ShaderFeatures::empty(), context)
        .expect(&format!("Failed to parse {}", path));
}

#[test]
fn newpixie_params() {
    let path = "../test/shaders_slang/crt/newpixie-crt.slangp";
    let mut context = WildcardContext::new();

    context.add_video_driver_defaults(VideoDriver::Vulkan);

    context.append_item(ContextItem::CoreName(String::from("image display")));

    let parsed = ShaderPreset::try_parse_with_context(path, ShaderFeatures::empty(), context)
        .expect(&format!("Failed to parse {}", path));

    println!("{:?}", parsed.parameters)
}
