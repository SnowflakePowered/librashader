use crate::error::ParsePresetError;
use std::convert::Infallible;
use std::path::PathBuf;
use std::str::FromStr;

#[repr(i32)]
#[derive(Default, Debug)]
pub enum FilterMode {
    #[default]
    Linear = 0,
    Nearest,
    Unspecified,
}

#[repr(i32)]
#[derive(Default, Debug)]
pub enum WrapMode {
    #[default]
    ClampToBorder = 0,
    ClampToEdge,
    Repeat,
    MirroredRepeat,
}

#[repr(i32)]
#[derive(Default, Debug)]
pub enum ScaleType {
    #[default]
    Input = 0,
    Absolute,
    Viewport,
}

#[derive(Debug)]
pub enum ScaleFactor {
    Float(f32),
    Absolute(i32),
}

impl Default for ScaleFactor {
    fn default() -> Self {
        ScaleFactor::Float(1.0f32)
    }
}

impl FromStr for WrapMode {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "clamp_to_border" => WrapMode::ClampToBorder,
            "clamp_to_edge" => WrapMode::ClampToEdge,
            "repeat" => WrapMode::Repeat,
            "mirrored_repeat" => WrapMode::MirroredRepeat,
            _ => WrapMode::ClampToBorder,
        })
    }
}

impl FromStr for ScaleType {
    type Err = ParsePresetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "source" => Ok(ScaleType::Input),
            "viewport" => Ok(ScaleType::Viewport),
            "absolute" => Ok(ScaleType::Absolute),
            _ => Err(ParsePresetError::InvalidScaleType(s.to_string())),
        }
    }
}

pub struct Scaling {
    pub scale_type: ScaleType,
    pub factor: ScaleFactor,
}

pub struct Scale2D {
    pub valid: bool,
    pub x: Scaling,
    pub y: Scaling,
}

pub struct ShaderConfig {
    pub id: usize,
    pub name: String,
    pub alias: String,
    pub filter: FilterMode,
    pub wrap_mode: WrapMode,
    pub frame_count_mod: u32,
    pub srgb_framebuffer: bool,
    pub float_framebuffer: bool,
    pub feedback_pass: i32,
    pub mipmap_input: bool,
    pub scaling: Scale2D,
}

pub struct TextureConfig {
    pub name: String,
    pub path: PathBuf,
    pub wrap_mode: WrapMode,
    pub filter: FilterMode,
    pub mipmap: bool,
}

pub struct Parameter {
    pub name: String,
    pub value: f32,
}

pub struct Preset {
    // Everything is in Vecs because the expect number of values is well below 64.
    pub shaders: Vec<ShaderConfig>,
    pub textures: Vec<TextureConfig>,
    pub parameters: Vec<Parameter>,
}
