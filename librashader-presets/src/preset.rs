use crate::error::ParsePresetError;
use librashader_common::{FilterMode, WrapMode};
use std::ops::Mul;
use std::path::PathBuf;
use std::str::FromStr;

/// The configuration for a single shader pass.
#[derive(Debug, Clone)]
pub struct ShaderPassConfig {
    /// The index of the shader pass relative to its parent preset.
    pub id: i32,
    /// The path to the shader pass source file.
    pub name: PathBuf,
    /// The alias of the shader pass if available.
    pub alias: Option<String>,
    /// The filtering mode that this shader pass should expect.
    pub filter: FilterMode,
    /// The texture addressing (wrap) mode that this shader pass expects.
    pub wrap_mode: WrapMode,
    /// The number to which to wrap the frame count before passing it to the uniforms.
    pub frame_count_mod: u32,
    /// Whether or not this shader pass expects an SRGB framebuffer output.
    pub srgb_framebuffer: bool,
    /// Whether or not this shader pass expects an float framebuffer output.
    pub float_framebuffer: bool,
    /// Whether or not to generate mipm
    pub mipmap_input: bool,
    pub scaling: Scale2D,
}

#[repr(i32)]
#[derive(Default, Copy, Clone, Debug)]
pub enum ScaleType {
    #[default]
    Input = 0,
    Absolute,
    Viewport,
}

#[derive(Copy, Clone, Debug)]
pub enum ScaleFactor {
    Float(f32),
    Absolute(i32),
}

impl Default for ScaleFactor {
    fn default() -> Self {
        ScaleFactor::Float(1.0f32)
    }
}

impl From<ScaleFactor> for f32 {
    fn from(value: ScaleFactor) -> Self {
        match value {
            ScaleFactor::Float(f) => f,
            ScaleFactor::Absolute(f) => f as f32,
        }
    }
}

impl Mul<ScaleFactor> for f32 {
    type Output = f32;

    fn mul(self, rhs: ScaleFactor) -> Self::Output {
        match rhs {
            ScaleFactor::Float(f) => f * self,
            ScaleFactor::Absolute(f) => f as f32 * self,
        }
    }
}

impl Mul<ScaleFactor> for u32 {
    type Output = f32;

    fn mul(self, rhs: ScaleFactor) -> Self::Output {
        match rhs {
            ScaleFactor::Float(f) => f * self as f32,
            ScaleFactor::Absolute(f) => (f as u32 * self) as f32,
        }
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

#[derive(Debug, Clone)]
pub struct Scaling {
    pub scale_type: ScaleType,
    pub factor: ScaleFactor,
}

#[derive(Debug, Clone)]
pub struct Scale2D {
    pub valid: bool,
    pub x: Scaling,
    pub y: Scaling,
}

#[derive(Debug, Clone)]
pub struct TextureConfig {
    pub name: String,
    pub path: PathBuf,
    pub wrap_mode: WrapMode,
    pub filter_mode: FilterMode,
    pub mipmap: bool,
}

#[derive(Debug, Clone)]
pub struct ParameterConfig {
    pub name: String,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub struct ShaderPreset {
    /// Used in legacy GLSL shader semantics. If < 0, no feedback pass is used.
    /// Otherwise, the FBO after pass #N is passed a texture to next frame
    #[cfg(feature = "parse_legacy_glsl")]
    pub feedback_pass: i32,

    /// The number of shaders enabled in the filter chain.
    pub shader_count: i32,
    // Everything is in Vecs because the expect number of values is well below 64.
    /// Preset information for each shader.
    pub shaders: Vec<ShaderPassConfig>,

    /// Preset information for each texture.
    pub textures: Vec<TextureConfig>,

    /// Preset information for each user parameter.
    pub parameters: Vec<ParameterConfig>,
}
