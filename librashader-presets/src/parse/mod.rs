use std::path::Path;

use nom_locate::LocatedSpan;
use std::str;

mod preset;
mod token;
mod value;

pub(crate) type Span<'a> = LocatedSpan<&'a str>;
pub(crate) use token::Token;

use crate::context::{VideoDriver, WildcardContext};
use crate::error::ParsePresetError;
use crate::parse::preset::resolve_values;
use crate::parse::value::parse_preset;
use crate::{ShaderFeatures, ShaderPreset};

pub(crate) fn remove_if<T>(values: &mut Vec<T>, f: impl FnMut(&T) -> bool) -> Option<T> {
    values.iter().position(f).map(|idx| values.remove(idx))
}

impl ShaderPreset {
    /// Try to parse the shader preset at the given path.
    ///
    /// This will add path defaults to the wildcard resolution context.
    pub fn try_parse(
        path: impl AsRef<Path>,
        shader_features: ShaderFeatures,
    ) -> Result<ShaderPreset, ParsePresetError> {
        let mut context = WildcardContext::new();
        context.add_path_defaults(path.as_ref());
        let values = parse_preset(path, context)?;
        Ok(resolve_values(values, shader_features))
    }

    /// Try to parse the shader preset at the given path.
    ///
    /// This will add path and driver defaults to the wildcard resolution context.
    pub fn try_parse_with_driver_context(
        path: impl AsRef<Path>,
        shader_features: ShaderFeatures,
        driver: VideoDriver,
    ) -> Result<ShaderPreset, ParsePresetError> {
        let mut context = WildcardContext::new();
        context.add_path_defaults(path.as_ref());
        context.add_video_driver_defaults(driver);
        let values = parse_preset(path, context)?;
        Ok(resolve_values(values, shader_features))
    }

    /// Try to parse the shader preset at the given path, with the exact provided context.
    ///
    /// This function does not change any of the values in the provided context, except calculating `VID-FINAL-ROT`
    /// if `CORE-REQ-ROT` and `VID-USER-ROT` is present.
    pub fn try_parse_with_context(
        path: impl AsRef<Path>,
        shader_features: ShaderFeatures,
        context: WildcardContext,
    ) -> Result<ShaderPreset, ParsePresetError> {
        let values = parse_preset(path, context)?;
        Ok(resolve_values(values, shader_features))
    }
}

#[cfg(test)]
mod test {
    use crate::ShaderPreset;
    use librashader_common::shader_features::ShaderFeatures;
    use std::path::PathBuf;

    #[test]
    pub fn parse_preset() {
        let root = PathBuf::from("../test/shaders_slang/ntsc/ntsc-256px-svideo.slangp");
        let basic = ShaderPreset::try_parse(root, ShaderFeatures::empty());
        eprintln!("{basic:#?}");
        assert!(basic.is_ok());
    }
}
