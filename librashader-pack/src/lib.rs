//! Shader preset resource handling for librashader.
//!
//! This crate contains facilities to load shader preset resources from a [`ShaderPreset`].
//!
//! Also defines abstractly the `.slangpkg` shader preset format implemented via serde derives on [`ShaderPresetPack`].
//!
use image::{ImageError, RgbaImage};
use librashader_preprocess::{PreprocessError, ShaderSource};
use librashader_presets::{
    ParameterMeta, PassMeta, PresetColorSpace, ShaderFeatures, ShaderPreset, TextureMeta,
};
use std::path::Path;

use librashader_common::ColorSpace;

/// A buffer holding RGBA image bytes.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextureBuffer {
    #[cfg_attr(feature = "serde", serde(with = "serde_base64_or_bytes"))]
    image: Vec<u8>,
    width: u32,
    height: u32,
}

impl From<TextureBuffer> for Option<RgbaImage> {
    fn from(value: TextureBuffer) -> Self {
        RgbaImage::from_raw(value.width, value.height, value.image)
    }
}

impl AsRef<[u8]> for TextureBuffer {
    fn as_ref(&self) -> &[u8] {
        self.image.as_ref()
    }
}

impl From<RgbaImage> for TextureBuffer {
    fn from(value: RgbaImage) -> Self {
        let width = value.width();
        let height = value.height();
        TextureBuffer {
            image: value.into_raw(),
            width,
            height,
        }
    }
}

/// A resource for a shader preset, fully loaded into memory.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LoadedResource<M: LoadableResource> {
    /// The fully qualified path to the resource.
    pub data: M::ResourceType,
    /// Meta information about the resource.
    pub meta: M,
}

/// Trait for a resource that is loadable from disk.
pub trait LoadableResource {
    /// The type of the resource.
    type ResourceType;
    /// The error type when loading the resource.
    type Error;
    /// The type of options to pass to the loader.
    type Options;
    /// Load the resource from the path.
    fn load(path: &Path, options: Self::Options) -> Result<Self::ResourceType, Self::Error>;
}

impl LoadableResource for PassMeta {
    type ResourceType = ShaderSource;
    type Error = PreprocessError;
    type Options = ShaderFeatures;

    fn load(path: &Path, options: Self::Options) -> Result<Self::ResourceType, Self::Error> {
        ShaderSource::load(path, options)
    }
}

impl LoadableResource for TextureMeta {
    type ResourceType = TextureBuffer;
    type Error = ImageError;
    type Options = ();

    fn load(path: &Path, _options: Self::Options) -> Result<Self::ResourceType, Self::Error> {
        image::open(path).map(|img| TextureBuffer::from(img.to_rgba8()))
    }
}

/// The loaded resource information for the source code of a shader pass.
pub type PassResource = LoadedResource<PassMeta>;

/// The loaded texture resource for a shader preset.
pub type TextureResource = LoadedResource<TextureMeta>;

/// The language that the shader sources are in.
///
/// In the vast majority of cases, this will always be GLSL.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ShaderSourceLanguage {
    /// GLSL
    Glsl,
    /// WGSL (only produced when creating a .wgsl.slangpkg
    Wgsl
}

/// A fully loaded-in-memory shader preset, with all paths resolved to data.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShaderPresetPack {
    /// The language that the pass shader sources are in.
    ///
    /// Almost always GLSL, unless created from a .wgsl.slangpkg
    pub language: ShaderSourceLanguage,

    /// Used in legacy GLSL shader semantics. If < 0, no feedback pass is used.
    /// Otherwise, the FBO after pass #N is passed a texture to next frame
    #[cfg(feature = "parse_legacy_glsl")]
    pub feedback_pass: i32,

    /// The number of shaders enabled in the filter chain.
    pub pass_count: i32,
    // Everything is in Vecs because the expect number of values is well below 64.
    /// Preset information for each shader.
    pub passes: Vec<PassResource>,

    /// Preset information for each texture.
    pub textures: Vec<TextureResource>,

    /// Preset information for each user parameter.
    pub parameters: Vec<ParameterMeta>,
}

#[cfg(feature = "load")]
impl ShaderPresetPack {
    /// Load a `ShaderPack` from a [`ShaderPreset`].
    pub fn load_from_preset<E>(preset: ShaderPreset) -> Result<ShaderPresetPack, E>
    where
        E: From<PreprocessError>,
        E: From<ImageError>,
        E: Send,
    {
        use rayon::prelude::*;

        let shaders_iter = preset.passes.into_par_iter();
        let textures_iter = preset.textures.into_par_iter();

        Ok(ShaderPresetPack {
            language: ShaderSourceLanguage::Glsl,

            #[cfg(feature = "parse_legacy_glsl")]
            feedback_pass: preset.feedback_pass,

            pass_count: preset.pass_count,
            passes: shaders_iter
                .map(|v| {
                    Ok::<_, E>(PassResource {
                        // The default PassMeta::load function is always GLSL.
                        data: PassMeta::load(v.path.as_path(), preset.features)?,
                        meta: v.meta,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            textures: textures_iter
                .into_par_iter()
                .map(|t| {
                    Ok::<_, E>(TextureResource {
                        data: TextureMeta::load(t.path.as_path(), ())?,
                        meta: t.meta,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            parameters: preset.parameters,
        })
    }
}

impl PresetColorSpace for ShaderPresetPack {
    /// Pack-level implementation: the shader source is already loaded so this
    /// is infallible — but we still return `Result` to share the trait
    /// signature with [`ShaderPreset`].
    fn color_space(&self) -> Result<ColorSpace, PreprocessError> {
        let Some(last) = self.passes.last() else {
            return Ok(ColorSpace::Sdr);
        };
        let effective_format = last.meta.get_format_override().unwrap_or(last.data.format);

        Ok(match effective_format {
            librashader_common::ImageFormat::A2B10G10R10UnormPack32 => ColorSpace::Hdr10,
            librashader_common::ImageFormat::R16G16B16A16Sfloat => ColorSpace::ScRgb,
            _ => ColorSpace::Sdr,
        })
    }
}

#[cfg(feature = "serde")]
mod serde_base64_or_bytes {
    use base64::display::Base64Display;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use serde::{Deserializer, Serializer};

    #[allow(clippy::ptr_arg)]
    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            s.collect_str(&Base64Display::new(v, &STANDARD))
        } else {
            serde_bytes::serialize(v, s)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        if d.is_human_readable() {
            struct Base64Visitor;
            impl<'de> serde::de::Visitor<'de> for Base64Visitor {
                type Value = Vec<u8>;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a base64 encoded string")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_bytes(v.as_ref())
                }

                fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_bytes(v.as_ref())
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    STANDARD.decode(v).map_err(serde::de::Error::custom)
                }

                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_bytes(v.as_ref())
                }
            }

            d.deserialize_str(Base64Visitor)
        } else {
            serde_bytes::deserialize(d)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ShaderPresetPack;
    use librashader_presets::{ShaderFeatures, ShaderPreset};
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test() {
        let preset = ShaderPreset::try_parse(
            "../test/shaders_slang/crt/crt-royale.slangp",
            ShaderFeatures::NONE,
        )
        .unwrap();
        let resolved = ShaderPresetPack::load_from_preset::<anyhow::Error>(preset).unwrap();
        let mut file = File::create("crt-royale.slangpack.json").unwrap();
        file.write_all(serde_json::to_vec_pretty(&resolved).unwrap().as_ref())
            .unwrap();
    }

    #[test]
    fn test_rmp() {
        let preset = ShaderPreset::try_parse(
            "../test/shaders_slang/crt/crt-royale.slangp",
            ShaderFeatures::NONE,
        )
        .unwrap();
        let resolved = ShaderPresetPack::load_from_preset::<anyhow::Error>(preset).unwrap();
        let mut file = File::create("crt-royale.slangpack").unwrap();
        file.write_all(rmp_serde::to_vec(&resolved).unwrap().as_ref())
            .unwrap();
    }
}
