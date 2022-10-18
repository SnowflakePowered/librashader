use std::collections::HashSet;
use std::convert::Infallible;
use std::path::PathBuf;
use std::str::FromStr;

#[repr(C)]
pub enum FilterMode {
    Linear,
    Nearest,
    Unspecified
}

#[repr(C)]
pub enum WrapMode {
    ClampToBorder,
    ClampToEdge,
    Repeat,
    MirroredRepeat,
}

#[repr(C)]
pub enum ScaleType {
    Input,
    Absolute,
    Viewport
}

impl FromStr for WrapMode {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "clamp_to_border" => WrapMode::ClampToBorder,
            "clamp_to_edge" => WrapMode::ClampToEdge,
            "repeat" => WrapMode::Repeat,
            "mirrored_repeat" => WrapMode::MirroredRepeat,
            _ => WrapMode::ClampToBorder
        })
    }
}

impl FromStr for ScaleType {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "source" => ScaleType::Input,
            "viewport" => ScaleType::Viewport,
            "absolute" => ScaleType::Absolute,
            _ => ScaleType::Input
        })
    }
}

pub enum ScaleFactor {
    Float(f32),
    Absolute(i32)
}

pub struct Scaling {
    pub scale_type: ScaleType,
    pub factor: ScaleFactor
}

pub struct Scale2D {
    pub x: Scaling,
    pub y: Scaling
}

pub struct ShaderConfig {
    pub name: String,
    pub alias: String,
    pub filter: FilterMode,
    pub wrap_mode: WrapMode,
    pub frame_count_mod: usize,
    pub srgb_framebuffer: bool,
    pub float_framebuffer: bool,
    pub mipmap_input: bool,
    pub scaling: Scale2D
}

pub struct TextureConfig {
    pub name: String,
    pub path: PathBuf,
    pub wrap_mode: WrapMode,
    pub filter: FilterMode,
    pub mipmap: bool
}

pub struct Parameter {
    pub name: String,
    pub value: f32,
}

pub struct Preset {
    // Everything is in Vecs because the expect number of values is well below 64.
    pub shaders: Vec<ShaderConfig>,
    pub textures: Vec<TextureConfig>,
    pub parameters: Vec<Parameter>
}