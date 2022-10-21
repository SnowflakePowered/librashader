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
use crate::Preset;

pub(crate) fn remove_if<T>(values: &mut Vec<T>, f: impl FnMut(&T) -> bool) -> Option<T> {
    values.iter().position(f).map(|idx| values.remove(idx))
}

impl Preset {
    pub fn try_parse(path: impl AsRef<Path>) -> Result<Preset, ParsePresetError> {
        let values = parse_preset(path)?;
        Ok(resolve_values(values))
    }
}

#[cfg(test)]
mod test {
    use crate::Preset;
    use std::path::PathBuf;

    #[test]
    pub fn parse_preset() {
        let root =
            PathBuf::from("../test/slang-shaders/ntsc/ntsc-256px-svideo.slangp");
        let basic = Preset::try_parse(root);
        eprintln!("{:#?}", basic);
        assert!(basic.is_ok());
    }
}
