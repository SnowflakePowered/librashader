use std::collections::HashMap;
use std::path::Path;

use nom_locate::LocatedSpan;
use std::str;

mod preset;
mod token;
mod value;

pub(crate) type Span<'a> = LocatedSpan<&'a str>;
pub(crate) use token::Token;

use crate::error::ParsePresetError;
use crate::parse::preset::resolve_values;
use crate::parse::value::parse_preset;
use crate::ShaderPreset;

pub(crate) fn remove_if<T>(values: &mut Vec<T>, f: impl FnMut(&T) -> bool) -> Option<T> {
    values.iter().position(f).map(|idx| values.remove(idx))
}

impl ShaderPreset {
    /// Try to parse the shader preset at the given path.
    pub fn try_parse(path: impl AsRef<Path>) -> Result<ShaderPreset, ParsePresetError> {
        ShaderPreset::try_parse_with_context(path, HashMap::new())
    }

    /// Try to parse the shader preset at the given path.
    pub fn try_parse_with_context(path: impl AsRef<Path>, context: HashMap<String, String>) -> Result<ShaderPreset, ParsePresetError> {
        let values = parse_preset(path, context)?;
        Ok(resolve_values(values))
    }
}

#[cfg(test)]
mod test {
    use crate::ShaderPreset;
    use std::path::PathBuf;

    #[test]
    pub fn parse_preset() {
        let root = PathBuf::from("../test/slang-shaders/ntsc/ntsc-256px-svideo.slangp");
        let basic = ShaderPreset::try_parse(root);
        eprintln!("{basic:#?}");
        assert!(basic.is_ok());
    }
}
