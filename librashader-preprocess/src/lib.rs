mod error;
mod include;
mod pragma;
mod stage;

use crate::include::read_source;
pub use error::*;
use librashader::ShaderSource;
use std::path::Path;

pub(crate) trait SourceOutput {
    fn push_line(&mut self, str: &str);
    fn mark_line(&mut self, line_no: usize, comment: &str) {
        #[cfg(feature = "line_directives")]
        self.push_line(&format!("#line {} \"{}\"", line_no, comment))
    }
}

impl SourceOutput for String {
    fn push_line(&mut self, str: &str) {
        self.push_str(str);
        self.push('\n');
    }
}

pub fn load_shader_source(path: impl AsRef<Path>) -> Result<ShaderSource, PreprocessError> {
    let source = read_source(path)?;
    let meta = pragma::parse_pragma_meta(&source)?;
    let text = stage::process_stages(&source)?;

    Ok(ShaderSource {
        vertex: text.vertex,
        fragment: text.fragment,
        name: meta.name,
        parameters: meta.parameters,
        format: meta.format,
    })
}

#[cfg(test)]
mod test {
    use crate::include::read_source;
    use crate::{load_shader_source, pragma};

    #[test]
    pub fn load_file() {
        let result = load_shader_source(
            "../test/slang-shaders/blurs/shaders/royale/blur3x3-last-pass.slang",
        )
        .unwrap();
        eprintln!("{:#}", result.vertex)
    }

    #[test]
    pub fn preprocess_file() {
        let result =
            read_source("../test/slang-shaders/blurs/shaders/royale/blur3x3-last-pass.slang")
                .unwrap();
        eprintln!("{result}")
    }

    #[test]
    pub fn get_param_pragmas() {
        let result = read_source(
            "../test/slang-shaders/crt/shaders/crt-maximus-royale/src/ntsc_pass1.slang",
        )
        .unwrap();

        let params = pragma::parse_pragma_meta(result).unwrap();
        eprintln!("{params:?}")
    }
}
