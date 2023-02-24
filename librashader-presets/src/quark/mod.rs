use std::f32;
use crate::parse::{ShaderStage, ShaderType, Value};
use crate::{ParseErrorKind, ParsePresetError, ScaleFactor, ScaleType};
use bml::BmlNode;
use librashader_common::{FilterMode, ImageFormat, WrapMode};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

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



fn parse_scale(scale: &str) -> Result<(ScaleType, ScaleFactor), ParsePresetError> {
    if scale.ends_with("%") {
        let value = f32::from_str(scale.trim_end_matches("%"))
            .map_err(|_| {
                eprintln!("{scale}");
                ParsePresetError::ParserError {
                    offset: 0,
                    row: 0,
                    col: 0,
                    kind: ParseErrorKind::UnsignedInt,
                }
            })? as f32 / 100.0;

        Ok((ScaleType::Input, ScaleFactor::Float(value)))
    } else {
        // allowed to end in " px"
        let value = i32::from_str(scale.trim_end_matches(" px"))
            .map_err(|_| {
                eprintln!("{scale}");
                ParsePresetError::ParserError {
                    offset: 0,
                    row: 0,
                    col: 0,
                    kind: ParseErrorKind::UnsignedInt,
                }
            })?;

        Ok((ScaleType::Absolute, ScaleFactor::Absolute(value)))
    }
}


fn parse_values(node: &BmlNode, root: impl AsRef<Path>) -> Result<Vec<Value>, ParsePresetError> {
    let mut values = Vec::new();

    for (index, (name, program)) in node.nodes().enumerate() {
        eprintln!("{}, {:?}", name, program);

        if let Some(filter) = program.named("filter").next() {
            // NOPANIC: infallible
            values.push(Value::FilterMode(
                index as i32,
                FilterMode::from_str(filter.value().trim()).unwrap(),
            ))
        }
        if let Some(wrap) = program.named("wrap").next() {
            values.push(Value::WrapMode(
                index as i32,
                WrapMode::from_str(wrap.value().trim()).unwrap(),
            ))
        }

        if let Some(height) = program.named("height").next() {
            let height = height.value().trim();
            let (scale_type, factor) = parse_scale(height)?;
            values.push(Value::ScaleTypeY(
                index as i32,
                scale_type,
            ));
            values.push(Value::ScaleY(
                index as i32,
                factor,
            ))
        } else if name != "input" {
            values.push(Value::ScaleTypeY(
                index as i32,
                ScaleType::Viewport,
            ))
        }

        if let Some(width) = program.named("width").next() {
            let width = width.value().trim();
            let (scale_type, factor) = parse_scale(width)?;
            values.push(Value::ScaleTypeY(
                index as i32,
                scale_type,
            ));
            values.push(Value::ScaleY(
                index as i32,
                factor,
            ))
        } else if name != "input" {
            values.push(Value::ScaleTypeY(
                index as i32,
                ScaleType::Viewport,
            ))
        }

        if let Some(format) = program.named("format").next() {
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


        if let Some(modulo) = program.named("modulo").next() {
            let modulo =
                u32::from_str(modulo.value()).map_err(|_| ParsePresetError::ParserError {
                    offset: index,
                    row: 0,
                    col: 0,
                    kind: ParseErrorKind::UnsignedInt,
                })?;
            values.push(Value::FrameCountMod(index as i32, modulo))
        }



        if let Some(vertex) = program.named("vertex").next() {
            let mut path = root.as_ref().to_path_buf();
            path.push(vertex.value());
            let path = path
                .canonicalize()
                .map_err(|e| ParsePresetError::IOError(path.to_path_buf(), e))?;

            values.push(Value::Shader(
                index as i32,
                ShaderType::Quark(ShaderStage::Vertex),
                path,
            ))
        }


        if let Some(fragment) = program.named("fragment").next() {
            let mut path = root.as_ref().to_path_buf();
            path.push(fragment.value());

            let path = path
                .canonicalize()
                .map_err(|e| ParsePresetError::IOError(path.to_path_buf(), e))?;

            values.push(Value::Shader(
                index as i32,
                ShaderType::Quark(ShaderStage::Fragment),
                path,
            ))
        }

        for (index, texture) in program.named("pixmap").enumerate() {
            let mut path = root.as_ref().to_path_buf();
            path.push(texture.value());
            let path = path
                .canonicalize()
                .map_err(|e| ParsePresetError::IOError(path.to_path_buf(), e))?;

            values.push(Value::Texture {
                name: index.to_string(),
                filter_mode: texture.named("filter")
                    .next()
                    .map(|filter| FilterMode::from_str(filter.value().trim()).unwrap())
                    .unwrap_or_default(),
                wrap_mode: texture.named("wrap")
                    .next()
                    .map(|wrap| WrapMode::from_str(wrap.value().trim()).unwrap())
                    .unwrap_or_default(),
                mipmap: false,
                path,
            });
        }
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shader() {
        let preset = parse_bml_node("../test/quark-shaders/CRT-Royale.shader").unwrap();
        let values = parse_values(&preset, "../test/quark-shaders/CRT-Royale.shader");
        eprintln!("{values:#?}");

    }
}
