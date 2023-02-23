use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use bml::BmlNode;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

pub fn parse_preset(path: impl AsRef<Path>) -> Result<BmlNode, Box<dyn Error>> {
    let path = path.as_ref();
    let path = path
        .canonicalize()?;

    let mut manifest_path = path.join("manifest.bml");
    let mut contents = String::new();
    File::open(&manifest_path)
        .and_then(|mut f| f.read_to_string(&mut contents))?;
    // BML expects a newline.
    contents.push_str("\n");
    let contents = contents.to_string();
    Ok(bml::BmlNode::try_from(&*contents)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shader() {
        let preset = parse_preset("../test/quark-shaders/CRT-Royale.shader").unwrap();

        for program in preset.named("program") {
            eprintln!("{:?}", program);

        }

    }
}
