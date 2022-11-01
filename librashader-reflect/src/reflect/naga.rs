use crate::error::ShaderReflectError;
use crate::front::naga::NagaCompilation;
use crate::front::shaderc::GlslangCompilation;
use naga::front::spv::Options;
use naga::Module;

#[derive(Debug)]
pub struct NagaReflect {
    vertex: Module,
    fragment: Module,
}

impl TryFrom<NagaCompilation> for NagaReflect {
    type Error = ShaderReflectError;

    fn try_from(value: NagaCompilation) -> Result<Self, Self::Error> {
        Ok(NagaReflect {
            vertex: value.vertex,
            fragment: value.fragment,
        })
    }
}

impl TryFrom<GlslangCompilation> for NagaReflect {
    type Error = ShaderReflectError;

    fn try_from(value: GlslangCompilation) -> Result<Self, Self::Error> {
        let ops = Options::default();
        let vertex =
            naga::front::spv::Parser::new(value.vertex.as_binary().to_vec().into_iter(), &ops)
                .parse()?;
        let fragment =
            naga::front::spv::Parser::new(value.fragment.as_binary().to_vec().into_iter(), &ops)
                .parse()?;
        Ok(NagaReflect { vertex, fragment })
    }
}

#[cfg(test)]
mod test {
    
    
    

    #[test]
    pub fn test_into() {
        let result = librashader_preprocess::load_shader_source("../test/basic.slang").unwrap();
        let _spirv = crate::front::shaderc::compile_spirv(&result).unwrap();
    }
}
