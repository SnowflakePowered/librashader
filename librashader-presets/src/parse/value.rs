use crate::{ParameterConfig, remove_if, Scale2D, ScaleFactor, ScaleType, Scaling, ShaderPassConfig, ShaderPath, ShaderPreset, TextureConfig};
use librashader_common::{FilterMode, ImageFormat, WrapMode};
use std::path::PathBuf;

#[derive(Debug)]
pub enum ShaderStage {
    Fragment,
    Vertex,
    Geometry
}

#[derive(Debug)]
pub enum ShaderType {
    Slang,
    Quark(ShaderStage)
}

#[derive(Debug)]
pub enum Value {
    ShaderCount(i32),
    FeedbackPass(i32),
    Shader(i32, ShaderType, PathBuf),
    ScaleX(i32, ScaleFactor),
    ScaleY(i32, ScaleFactor),
    Scale(i32, ScaleFactor),
    ScaleType(i32, ScaleType),
    ScaleTypeX(i32, ScaleType),
    ScaleTypeY(i32, ScaleType),
    FilterMode(i32, FilterMode),
    WrapMode(i32, WrapMode),
    FrameCountMod(i32, u32),
    FloatFramebuffer(i32, bool),
    SrgbFramebuffer(i32, bool),
    MipmapInput(i32, bool),
    Alias(i32, String),
    Parameter(String, f32),
    FormatOverride(i32, ImageFormat),
    Texture {
        name: String,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmap: bool,
        path: PathBuf,
    },
}

impl Value {
    pub(crate) fn shader_index(&self) -> Option<i32> {
        match self {
            Value::Shader(i, _, _) => Some(*i),
            Value::ScaleX(i, _) => Some(*i),
            Value::ScaleY(i, _) => Some(*i),
            Value::Scale(i, _) => Some(*i),
            Value::ScaleType(i, _) => Some(*i),
            Value::ScaleTypeX(i, _) => Some(*i),
            Value::ScaleTypeY(i, _) => Some(*i),
            Value::FilterMode(i, _) => Some(*i),
            Value::WrapMode(i, _) => Some(*i),
            Value::FrameCountMod(i, _) => Some(*i),
            Value::FloatFramebuffer(i, _) => Some(*i),
            Value::SrgbFramebuffer(i, _) => Some(*i),
            Value::MipmapInput(i, _) => Some(*i),
            Value::Alias(i, _) => Some(*i),
            Value::FormatOverride(i, _) => Some(*i),
            _ => None,
        }
    }
}

pub fn resolve_values(mut values: Vec<Value>) -> ShaderPreset {
    let textures: Vec<TextureConfig> = values
        .drain_filter(|f| matches!(*f, Value::Texture { .. }))
        .map(|value| {
            if let Value::Texture {
                name,
                filter_mode,
                wrap_mode,
                mipmap,
                path,
            } = value
            {
                TextureConfig {
                    name,
                    path,
                    wrap_mode,
                    filter_mode,
                    mipmap,
                }
            } else {
                unreachable!("values should all be of type Texture")
            }
        })
        .collect();
    let parameters: Vec<ParameterConfig> = values
        .drain_filter(|f| matches!(*f, Value::Parameter { .. }))
        .map(|value| {
            if let Value::Parameter(name, value) = value {
                ParameterConfig { name, value }
            } else {
                unreachable!("values should be all of type parameters")
            }
        })
        .collect();

    let mut shaders = Vec::new();
    let shader_count =
        remove_if(&mut values, |v| matches!(*v, Value::ShaderCount(_))).map_or(0, |value| {
            if let Value::ShaderCount(count) = value {
                count
            } else {
                unreachable!("value should be of type shader_count")
            }
        });

    #[cfg(feature = "parse_legacy_glsl")]
    let feedback_pass = remove_if(&mut values, |v| matches!(*v, Value::FeedbackPass(_)))
        .map(|value| {
            if let Value::FeedbackPass(pass) = value {
                pass
            } else {
                unreachable!("value should be of type feedback_pass")
            }
        })
        .unwrap_or(0);

    for shader in 0..shader_count {
        if let Some(Value::Shader(id, ShaderType::Slang, name)) = remove_if(
            &mut values,
            |v| matches!(*v, Value::Shader(shader_index, ShaderType::Slang, _) if shader_index == shader),
        ) {
            let shader_values: Vec<Value> = values
                .drain_filter(|v| v.shader_index() == Some(shader))
                .collect();
            let scale_type = shader_values.iter().find_map(|f| match f {
                Value::ScaleType(_, value) => Some(*value),
                _ => None,
            });

            let mut scale_type_x = shader_values.iter().find_map(|f| match f {
                Value::ScaleTypeX(_, value) => Some(*value),
                _ => None,
            });

            let mut scale_type_y = shader_values.iter().find_map(|f| match f {
                Value::ScaleTypeY(_, value) => Some(*value),
                _ => None,
            });

            if scale_type.is_some() {
                // scale takes priority
                // https://github.com/libretro/RetroArch/blob/fcbd72dbf3579eb31721fbbf0d89a139834bcce9/gfx/video_shader_parse.c#L310
                scale_type_x = scale_type;
                scale_type_y = scale_type;
            }

            let scale_valid = scale_type_x.is_some() || scale_type_y.is_some();

            let scale = shader_values.iter().find_map(|f| match f {
                Value::Scale(_, value) => Some(*value),
                _ => None,
            });

            let mut scale_x = shader_values.iter().find_map(|f| match f {
                Value::ScaleX(_, value) => Some(*value),
                _ => None,
            });

            let mut scale_y = shader_values.iter().find_map(|f| match f {
                Value::ScaleY(_, value) => Some(*value),
                _ => None,
            });

            if scale.is_some() {
                // scale takes priority
                // https://github.com/libretro/RetroArch/blob/fcbd72dbf3579eb31721fbbf0d89a139834bcce9/gfx/video_shader_parse.c#L310
                scale_x = scale;
                scale_y = scale;
            }

            let srgb_frambuffer = shader_values
                .iter()
                .find_map(|f| match f {
                    Value::SrgbFramebuffer(_, value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(false);

            let float_framebuffer = shader_values
                .iter()
                .find_map(|f| match f {
                    Value::FloatFramebuffer(_, value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(false);

            let framebuffer_format = if srgb_frambuffer {
                Some(ImageFormat::R8G8B8A8Srgb)
            } else if float_framebuffer {
                Some(ImageFormat::R16G16B16A16Sfloat)
            } else {
                None
            };

            let shader = ShaderPassConfig {
                id,
                source_path: ShaderPath::Slang(name),
                alias: shader_values.iter().find_map(|f| match f {
                    Value::Alias(_, value) => Some(value.to_string()),
                    _ => None,
                }),
                filter: shader_values
                    .iter()
                    .find_map(|f| match f {
                        Value::FilterMode(_, value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or_default(),
                wrap_mode: shader_values
                    .iter()
                    .find_map(|f| match f {
                        Value::WrapMode(_, value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or_default(),
                frame_count_mod: shader_values
                    .iter()
                    .find_map(|f| match f {
                        Value::FrameCountMod(_, value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0),
                framebuffer_format_override: framebuffer_format,
                mipmap_input: shader_values
                    .iter()
                    .find_map(|f| match f {
                        Value::MipmapInput(_, value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(false),
                scaling: Scale2D {
                    valid: scale_valid,
                    x: Scaling {
                        scale_type: scale_type_x.unwrap_or_default(),
                        factor: scale_x.unwrap_or_default(),
                    },
                    y: Scaling {
                        scale_type: scale_type_y.unwrap_or_default(),
                        factor: scale_y.unwrap_or_default(),
                    },
                },
            };

            shaders.push(shader)
        }
    }

    ShaderPreset {
        #[cfg(feature = "parse_legacy_glsl")]
        feedback_pass,
        shader_count,
        shaders,
        textures,
        parameters,
    }
}
