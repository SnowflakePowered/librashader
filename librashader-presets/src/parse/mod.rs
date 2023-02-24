use std::path::Path;

mod value;

pub(crate) use value::Value;
pub(crate) use value::ShaderType;
pub(crate) use value::ShaderStage;

use crate::error::ParsePresetError;
use value::resolve_values;
use crate::slang::parse_preset;
use crate::ShaderPreset;

impl ShaderPreset {
    /// Try to parse the shader preset at the given path.
    pub fn try_parse(path: impl AsRef<Path>) -> Result<ShaderPreset, ParsePresetError> {
        let values = parse_preset(path)?;
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
