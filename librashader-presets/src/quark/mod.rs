use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use bml::BmlNode;
use librashader_common::{FilterMode, ImageFormat, WrapMode};
use crate::parse::{ShaderStage, ShaderType, Value};
use crate::{ParseErrorKind, ParsePresetError};

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
        if let Some(wrap) = programs.named("wrap").next() {
            values.push(Value::WrapMode(index as i32, WrapMode::from_str(wrap.value().trim()).unwrap()))
        }
        if let Some(format) = programs.named("format").next() {
            let format = match format.value() {
                "rgba8" => ImageFormat::R8G8B8A8Unorm,
                "rgb10a2" => ImageFormat::A2B10G10R10UnormPack32,
                "rgba16" => ImageFormat::R16G16B16A16Sint,
                "rgba16f" => ImageFormat::R16G16B16A16Sfloat,
                "rgba32f" => ImageFormat::R32G32B32A32Sfloat,

                // srgb extension
                "srgb8" => ImageFormat::R8G8B8A8Srgb,

                // don't support rgba12
                _ => ImageFormat::Unknown,
            };

            values.push(Value::FormatOverride(index as i32, format));
        }
        if let Some(modulo) = programs.named("modulo").next() {
            let modulo = u32::from_str(modulo.value())
                .map_err(|_| ParsePresetError::ParserError {
                    offset: index,
                    row: 0,
                    col: 0,
                    kind: ParseErrorKind::UnsignedInt,
                })?;
            values.push(Value::FrameCountMod(index as i32, modulo))
        }
        if let Some(vertex) = programs.named("vertex").next() {
            let path = PathBuf::from_str(vertex.value().trim())
                .expect("Infallible");
            let path = path.canonicalize()
                .map_err(|e| ParsePresetError::IOError(path.to_path_buf(), e))?;

            values.push(Value::Shader(index as i32, ShaderType::Quark(ShaderStage::Vertex), path))
        }
        if let Some(fragment) = programs.named("fragment").next() {
            let path = PathBuf::from_str(fragment.value().trim())
                .expect("Infallible");
            let path = path.canonicalize()
                .map_err(|e| ParsePresetError::IOError(path.to_path_buf(), e))?;


            values.push(Value::Shader(index as i32, ShaderType::Quark(ShaderStage::Fragment), path))
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