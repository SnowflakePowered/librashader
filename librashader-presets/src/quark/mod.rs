use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use bml::BmlNode;
use librashader_common::FilterMode;
use crate::parse::{ShaderStage, ShaderType, Value};
use crate::ParsePresetError;

fn parse_bml_node(path: impl AsRef<Path>) -> Result<BmlNode, ParsePresetError> {
    let path = path.as_ref();
    let path = path
        .canonicalize()
        .map_err(|e| ParsePresetError::IOError(path.to_path_buf(), e))?;

    let mut manifest_path = path.join("manifest.bml");
    let mut contents = String::new();
    File::open(&manifest_path)
        .and_then(|mut f| f.read_to_string(&mut contents))
        .map_err(|e| ParsePresetError::IOError(path.to_path_buf(), e))?;
    // BML expects a newline.
    contents.push_str("\n");
    let contents = contents.to_string();
    Ok(bml::BmlNode::try_from(&*contents)?)
}

fn parse_values(node: &BmlNode) -> Result<Vec<Value>, ParsePresetError>{
    let programs = node.named("program");
    let program_len = programs.len();
    let mut values = Vec::new();
    for (index, programs) in programs.chain(node.named("output")).enumerate() {
        if let Some(filter) = programs.named("filter").next() {
            // NOPANIC: infallible
            values.push(Value::FilterMode(index as i32, FilterMode::from_str(filter.value().trim()).unwrap()))
        }
        if let Some(vertex) = programs.named("vertex").next() {
            values.push(Value::Shader(index as i32, ShaderType::Quark(ShaderStage::Vertex), PathBuf::from_str(vertex.value().trim())
                .expect("Infallible")))
        }
        if let Some(fragment) = programs.named("fragment").next() {
            values.push(Value::Shader(index as i32, ShaderType::Quark(ShaderStage::Fragment), PathBuf::from_str(fragment.value().trim())
                .expect("Infallible")))
        }

    }


    eprintln!("{values:?}");

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shader() {
        let preset = parse_bml_node("../test/quark-shaders/CRT-Royale.shader").unwrap();
        let values = parse_values(&preset);
        for program in preset.named("program").chain(preset.named("output")) {
            eprintln!("{:?}", program);

        }

    }
}