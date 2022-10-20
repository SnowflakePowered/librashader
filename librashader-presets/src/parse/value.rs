use crate::error::{ParseErrorKind, ParsePresetError};
use crate::parse::{Span, Token};
use crate::{FilterMode, ScaleFactor, ScaleType, Scaling, WrapMode};
use nom::bytes::complete::tag;
use nom::character::complete::digit1;
use nom::combinator::{eof, map_res};
use nom::error::Error;
use nom::IResult;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug)]
pub enum Value {
    ShaderCount(i32),
    Shader(i32, PathBuf),
    ScaleX(i32, ScaleFactor),
    ScaleY(i32, ScaleFactor),
    Scale(i32, ScaleFactor),
    ScaleType(i32, ScaleType),
    FilterMode(i32, FilterMode),
    FloatFramebuffer(i32, bool),
    SrgbFramebuffer(i32, bool),
    MipmapInput(i32, bool),
    WrapMode(i32, WrapMode),
    Alias(i32, String),
    Parameter(String, f32),
    Texture {
        name: String,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmap: bool,
        path: PathBuf,
    },
}

fn from_int(input: Span) -> Result<i32, ParsePresetError> {
    i32::from_str(input.trim()).map_err(|_| ParsePresetError::ParserError {
        offset: input.location_offset(),
        row: input.location_line(),
        col: input.get_column(),
        kind: ParseErrorKind::Int,
    })
}

fn from_float(input: Span) -> Result<f32, ParsePresetError> {
    f32::from_str(input.trim()).map_err(|_| ParsePresetError::ParserError {
        offset: input.location_offset(),
        row: input.location_line(),
        col: input.get_column(),
        kind: ParseErrorKind::Float,
    })
}

fn from_bool(input: Span) -> Result<bool, ParsePresetError> {
    if let Ok(i) = i32::from_str(input.trim()) {
        return match i {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err(ParsePresetError::ParserError {
                offset: input.location_offset(),
                row: input.location_line(),
                col: input.get_column(),
                kind: ParseErrorKind::Bool,
            }),
        };
    }
    bool::from_str(input.trim()).map_err(|_| ParsePresetError::ParserError {
        offset: input.location_offset(),
        row: input.location_line(),
        col: input.get_column(),
        kind: ParseErrorKind::Bool,
    })
}

fn parse_indexed_key<'a>(key: &'static str, input: Span<'a>) -> IResult<Span<'a>, i32> {
    let (input, _) = tag(key)(input)?;
    let (input, idx) = map_res(digit1, from_int)(input)?;
    let (input, _) = eof(input)?;
    Ok((input, idx))
}

fn parse_texture_key<'a, 'b>(
    key: &'static str,
    texture_name: &'b str,
    input: Span<'a>,
) -> IResult<Span<'a>, ()> {
    let (input, _) = tag(texture_name)(input)?;
    let (input, _) = tag("_")(input)?;
    let (input, _) = tag(key)(input)?;
    let (input, _) = eof(input)?;
    Ok((input, ()))
}

pub const SHADER_MAX_REFERENCE_DEPTH: usize = 16;

fn load_child_reference_strings(
    mut root_references: Vec<PathBuf>,
    root_path: impl AsRef<Path>,
) -> Result<Vec<(PathBuf, String)>, ParsePresetError> {
    let root_path = root_path.as_ref();
    let mut reference_depth = 0;
    let mut reference_strings: Vec<(PathBuf, String)> = Vec::new();
    while let Some(reference_path) = root_references.pop() {
        if reference_depth > SHADER_MAX_REFERENCE_DEPTH {
            return Err(ParsePresetError::ExceededReferenceDepth);
        }
        let mut root_path = root_path.to_path_buf();
        root_path.push(reference_path);
        let mut reference_root = root_path
            .canonicalize()
            .map_err(|e| ParsePresetError::IOError(root_path, e))?;

        let mut reference_contents = String::new();
        File::open(&reference_root)
            .map_err(|e| ParsePresetError::IOError(reference_root.clone(), e))?
            .read_to_string(&mut reference_contents)
            .map_err(|e| ParsePresetError::IOError(reference_root.clone(), e))?;

        let mut new_tokens = super::do_lex(&reference_contents)?;
        let mut new_references: Vec<PathBuf> = new_tokens
            .drain_filter(|token| *token.key.fragment() == "#reference")
            .map(|value| PathBuf::from(*value.value.fragment()))
            .collect();

        root_references.append(&mut new_references);

        // return the relative root that shader and texture paths are to be resolved against.
        if !reference_root.is_dir() {
            reference_root.pop();
        }

        // trim end space
        reference_strings.push((reference_root, reference_contents));
    }

    Ok(reference_strings)
}

pub fn parse_preset(path: impl AsRef<Path>) -> Result<Vec<Value>, ParsePresetError> {
    let path = path.as_ref();
    let path = path
        .canonicalize()
        .map_err(|e| ParsePresetError::IOError(path.to_path_buf(), e))?;

    let mut contents = String::new();
    File::open(&path)
        .and_then(|mut f| f.read_to_string(&mut contents))
        .map_err(|e| ParsePresetError::IOError(path.to_path_buf(), e))?;

    let tokens = super::token::do_lex(&contents)?;
    parse_values(tokens, path)
}

pub fn parse_values(
    mut tokens: Vec<Token>,
    root_path: impl AsRef<Path>,
) -> Result<Vec<Value>, ParsePresetError> {
    let mut root_path = root_path.as_ref().to_path_buf();
    if root_path.is_relative() {
        return Err(ParsePresetError::RootPathWasNotAbsolute);
    }
    if !root_path.is_dir() {
        // we don't really care if this doesn't do anything because a non-canonical root path will
        // fail at a later stage during resolution.
        root_path.pop();
    }

    let references: Vec<PathBuf> = tokens
        .drain_filter(|token| *token.key.fragment() == "#reference")
        .map(|value| PathBuf::from(*value.value.fragment()))
        .collect();

    // unfortunately we need to lex twice because there's no way to know the references ahead of time.
    let child_strings = load_child_reference_strings(references, &root_path)?;
    let mut all_tokens: Vec<(&Path, Vec<Token>)> = Vec::new();
    all_tokens.push((root_path.as_path(), tokens));

    for (path, string) in child_strings.iter() {
        let mut tokens = crate::parse::do_lex(string.as_ref())?;
        tokens.retain(|token| *token.key.fragment() != "#reference");
        all_tokens.push((path.as_path(), tokens))
    }

    // collect all possible parameter names.
    let mut parameter_names: Vec<&str> = Vec::new();
    for (_, tokens) in all_tokens.iter_mut() {
        for token in tokens.drain_filter(|token| *token.key.fragment() == "parameters") {
            let parameter_name_string: &str = *token.value.fragment();
            for parameter_name in parameter_name_string.split(";") {
                parameter_names.push(parameter_name);
            }
        }
    }

    // collect all possible texture names.
    let mut texture_names: Vec<&str> = Vec::new();
    for (_, tokens) in all_tokens.iter_mut() {
        for token in tokens.drain_filter(|token| *token.key.fragment() == "textures") {
            let texture_name_string: &str = *token.value.fragment();
            for texture_name in texture_name_string.split(";") {
                texture_names.push(texture_name);
            }
        }
    }

    let mut values = Vec::new();
    // resolve shader paths.
    for (ref path, tokens) in all_tokens.iter_mut() {
        for token in tokens.drain_filter(|token| parse_indexed_key("shader", token.key).is_ok()) {
            let (_, index) = parse_indexed_key("shader", token.key).map_err(|e| match e {
                nom::Err::Error(e) | nom::Err::Failure(e) => {
                    let input: Span = e.input;
                    ParsePresetError::ParserError {
                        offset: input.location_offset(),
                        row: input.location_line(),
                        col: input.get_column(),
                        kind: ParseErrorKind::Index("shader"),
                    }
                }
                _ => ParsePresetError::ParserError {
                    offset: 0,
                    row: 0,
                    col: 0,
                    kind: ParseErrorKind::Index("shader"),
                },
            })?;

            let mut relative_path = path.to_path_buf();
            relative_path.push(*token.value.fragment());
            relative_path
                .canonicalize()
                .map_err(|e| ParsePresetError::IOError(relative_path.clone(), e))?;
            values.push(Value::Shader(index, relative_path))
        }
    }

    // resolve texture paths
    let mut textures = Vec::new();
    for (ref path, tokens) in all_tokens.iter_mut() {
        for token in tokens.drain_filter(|token| texture_names.contains(token.key.fragment())) {
            let mut relative_path = path.to_path_buf();
            relative_path.push(*token.value.fragment());
            relative_path
                .canonicalize()
                .map_err(|e| ParsePresetError::IOError(relative_path.clone(), e))?;
            textures.push((token.key, relative_path))
        }
    }

    let mut tokens: Vec<Token> = all_tokens
        .into_iter()
        .flat_map(|(_, token)| token)
        .collect();

    for (texture, path) in textures {
        let mipmap = tokens
            .iter()
            .position(|t| {
                t.key.starts_with(*texture)
                    && t.key.ends_with("_mipmap")
                    && t.key.len() == texture.len() + "_mipmap".len()
            })
            .map(|idx| tokens.remove(idx))
            .map_or_else(|| Ok(false), |v| from_bool(v.value))?;

        let linear = tokens
            .iter()
            .position(|t| {
                t.key.starts_with(*texture)
                    && t.key.ends_with("_linear")
                    && t.key.len() == texture.len() + "_linear".len()
            })
            .map(|idx| tokens.remove(idx))
            .map_or_else(|| Ok(false), |v| from_bool(v.value))?;

        let wrap_mode = tokens
            .iter()
            .position(|t| {
                t.key.starts_with(*texture)
                    && t.key.ends_with("_wrap_mode")
                    && t.key.len() == texture.len() + "_wrap_mode".len()
            })
            .map(|idx| tokens.remove(idx))
            // NOPANIC: infallible
            .map_or_else(
                || WrapMode::default(),
                |v| WrapMode::from_str(*v.value).unwrap(),
            );

        values.push(Value::Texture {
            name: texture.to_string(),
            filter_mode: if linear {
                FilterMode::Linear
            } else {
                FilterMode::Nearest
            },
            wrap_mode,
            mipmap,
            path,
        })
    }
    for token in &tokens {
        if parameter_names.contains(token.key.fragment()) {
            let param_val = from_float(token.value)?;
            values.push(Value::Parameter(
                token.key.fragment().to_string(),
                param_val,
            ));
            continue;
        }
        // todo: handle shader props

        // handle undeclared parameters after parsing everything else as a last resort.
        let param_val = from_float(token.value)?;
        values.push(Value::Parameter(
            token.key.fragment().to_string(),
            param_val,
        ));
    }

    eprintln!("{:?}", tokens);
    // all tokens should be ok to process now.
    Ok(values)
}

#[cfg(test)]
mod test {
    use crate::parse::value::parse_preset;
    use std::path::PathBuf;

    #[test]
    pub fn parse_basic() {
        let root =
            PathBuf::from("test/slang-shaders/bezel/Mega_Bezel/Presets/Base_CRT_Presets/MBZ__3__STD__MEGATRON-NTSC.slangp");
        let basic = parse_preset(&root);
        eprintln!("{:?}", basic);
        assert!(basic.is_ok());
    }
}
