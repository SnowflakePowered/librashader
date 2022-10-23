use crate::error::ShaderReflectError;
use crate::front::shaderc::GlslangCompilation;
use spirv_cross::spirv::{Ast, Module};
use std::fmt::Debug;

pub struct SpirvCrossReflect<T>
where
    T: spirv_cross::spirv::Target,
{
    vertex: Ast<T>,
    fragment: Ast<T>,
}

impl<T> TryFrom<GlslangCompilation> for SpirvCrossReflect<T>
where
    T: spirv_cross::spirv::Target,
    Ast<T>: spirv_cross::spirv::Compile<T>,
    Ast<T>: spirv_cross::spirv::Parse<T>,
{
    type Error = ShaderReflectError;

    fn try_from(value: GlslangCompilation) -> Result<Self, Self::Error> {
        let vertex_module = Module::from_words(value.vertex.as_binary());
        let fragment_module = Module::from_words(value.fragment.as_binary());

        let vertex = Ast::parse(&vertex_module)?;
        let fragment = Ast::parse(&fragment_module)?;
        Ok(SpirvCrossReflect { vertex, fragment })
    }
}

#[cfg(test)]
mod test {
    use crate::reflect::spirv_cross::SpirvCrossReflect;
    use spirv_cross::glsl;

    #[test]
    pub fn test_into() {
        let result = librashader_preprocess::load_shader_source(
            "../test/slang-shaders/blurs/shaders/royale/blur3x3-last-pass.slang",
        )
        .unwrap();
        let spirv = crate::front::shaderc::compile_spirv(&result).unwrap();
        SpirvCrossReflect::<glsl::Target>::try_from(spirv).unwrap();
    }
}
