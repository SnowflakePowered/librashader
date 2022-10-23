use crate::error::ShaderCompileError;
use librashader::ShaderSource;
use naga::front::glsl::{Options, Parser};
use naga::{Module, ShaderStage};

#[derive(Debug)]
pub struct NagaCompilation {
    pub(crate) vertex: Module,
    pub(crate) fragment: Module,
}

pub fn compile_spirv(source: &ShaderSource) -> Result<NagaCompilation, ShaderCompileError> {
    let mut parser = Parser::default();
    let vertex = parser.parse(&Options::from(ShaderStage::Vertex), &source.vertex)?;
    let fragment = parser.parse(&Options::from(ShaderStage::Fragment), &source.fragment)?;
    Ok(NagaCompilation { vertex, fragment })
}

#[cfg(test)]
mod test {
    use crate::front::naga::compile_spirv;
    use naga::front::glsl::{Options, Parser};
    use naga::ShaderStage;

    #[test]
    pub fn compile_naga_test() {
        let result = librashader_preprocess::load_shader_source(
            "../test/slang-shaders/blurs/shaders/royale/blur3x3-last-pass.slang",
        )
        .unwrap();

        let fragment_source = result.fragment;
        let mut parser = Parser::default();
        println!("{fragment_source}");
        let fragment = parser
            .parse(&Options::from(ShaderStage::Fragment), &fragment_source)
            .unwrap();
    }

    #[test]
    pub fn compile_shader() {
        let result = librashader_preprocess::load_shader_source(
            "../test/slang-shaders/blurs/shaders/royale/blur3x3-last-pass.slang",
        )
        .unwrap();
        eprintln!("{:#}", &result.fragment[2930..2945]);
        let spirv = compile_spirv(&result).unwrap();
        eprintln!("{:?}", spirv)
    }
}
