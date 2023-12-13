use crate::error::ShaderReflectError;

use naga::Module;

use crate::reflect::semantics::ShaderSemantics;
use crate::reflect::{ReflectShader, ShaderReflection};

#[derive(Debug)]
pub struct NagaReflect {
    pub(crate) vertex: Module,
    pub(crate) fragment: Module,
}

//
// struct UboData {
//     // id: u32,
//     // descriptor_set: u32,
//     binding: u32,
//     size: u32,
// }
//
// struct Ubo {
//     members: Vec<StructMember>,
//     span: u32,
// }
//
// impl TryFrom<naga::Type> for Ubo {
//     type Error = Infallible;
//
//     fn try_from(value: Type) -> Result<Self, Infallible> {
//         match value.inner {
//             TypeInner::Struct { members, span } => Ok(Ubo { members, span }),
//             // todo: make this programmer error
//             _ => panic!(),
//         }
//     }
// }

impl NagaReflect {

}

impl ReflectShader for NagaReflect {
    fn reflect(
        &mut self,
        pass_number: usize,
        semantics: &ShaderSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError> {
        todo!()
    }
}

#[cfg(test)]
mod test {

    // #[test]
    // pub fn test_into() {
    //     let result = ShaderSource::load("../test/slang-shaders/crt/shaders/crt-royale/src/crt-royale-scanlines-horizontal-apply-mask.slang").unwrap();
    //     let compilation = crate::front::GlslangCompilation::try_from(&result).unwrap();
    //
    //     let mut loader = rspirv::dr::Loader::new();
    //     rspirv::binary::parse_words(compilation.vertex.as_binary(), &mut loader).unwrap();
    //     let module = loader.module();
    //
    //     let outputs: Vec<&Instruction> = module
    //         .types_global_values
    //         .iter()
    //         .filter(|i| i.class.opcode == Op::Variable)
    //         .collect();
    //
    //     println!("{outputs:#?}");
    // }
}
