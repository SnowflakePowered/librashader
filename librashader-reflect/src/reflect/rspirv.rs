use crate::error::ShaderReflectError;
use crate::front::shaderc::GlslangCompilation;
use rspirv_reflect::Reflection;
use shaderc::CompilationArtifact;

pub struct RspirvReflect {
    vertex: Reflection,
    fragment: Reflection,
}

fn parse_reflection(artifact: CompilationArtifact) -> Result<Reflection, ShaderReflectError> {
    let mut loader = rspirv::dr::Loader::new();
    rspirv::binary::parse_words(artifact.as_binary(), &mut loader)?;
    Ok(Reflection::new(loader.module()))
}

impl TryFrom<GlslangCompilation> for RspirvReflect {
    type Error = ShaderReflectError;

    fn try_from(value: GlslangCompilation) -> Result<Self, Self::Error> {
        let vertex = parse_reflection(value.vertex)?;
        let fragment = parse_reflection(value.fragment)?;

        Ok(RspirvReflect { vertex, fragment })
    }
}
#[cfg(test)]
mod test {
    use crate::reflect::rspirv::RspirvReflect;

    #[test]
    pub fn test_into() {
        let result = librashader_preprocess::load_shader_source("../test/basic.slang").unwrap();
        let spirv = crate::front::shaderc::compile_spirv(&result).unwrap();
        let mut reflect = RspirvReflect::try_from(spirv).unwrap();
        // let pcr = reflect.fragment.get_push_constant_range().unwrap()
        //     .unwrap();
        println!("{:?}", reflect.fragment.get_descriptor_sets());
        // println!("PushConstantInfo size: {}, off: {}",  pcr.size, pcr.offset);
    }
}
