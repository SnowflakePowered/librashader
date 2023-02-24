use crate::{ScaleFactor, ScaleType};
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
