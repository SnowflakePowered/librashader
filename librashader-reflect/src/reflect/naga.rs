use crate::error::{SemanticsErrorKind, ShaderReflectError};
use crate::front::NagaCompilation;
use crate::front::GlslangCompilation;
use crate::reflect::helper::SemanticErrorBlame;
use crate::reflect::semantics::MAX_BINDINGS_COUNT;
use naga::front::spv::Options;
use naga::{Arena, GlobalVariable, Handle, Module, ResourceBinding, StructMember, Type, TypeInner};
use std::convert::Infallible;

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

struct UboData {
    // id: u32,
    // descriptor_set: u32,
    binding: u32,
    size: u32,
}

struct Ubo {
    members: Vec<StructMember>,
    span: u32,
}

impl TryFrom<naga::Type> for Ubo {
    type Error = Infallible;

    fn try_from(value: Type) -> Result<Self, Infallible> {
        match value.inner {
            TypeInner::Struct { members, span } => Ok(Ubo { members, span }),
            // todo: make this programmer error
            _ => panic!(),
        }
    }
}

impl NagaReflect {
    // pub fn get_ubo_data(arena: Arena, variable: GlobalVariable, blame: SemanticErrorBlame) -> Result<UboData, ShaderReflectError> {
    //     let binding = match variable.binding {
    //         Some(ResourceBinding { group: 0, binding }) => binding,
    //         Some(ResourceBinding { group, .. }) => return Err(blame.error(SemanticsErrorKind::InvalidDescriptorSet(group))),
    //         None => return Err(blame.error(SemanticsErrorKind::InvalidDescriptorSet(u32::MAX))),
    //     };
    //
    //     if binding >= MAX_BINDINGS_COUNT {
    //         return Err(blame.error(SemanticsErrorKind::InvalidBinding(binding)));
    //     }
    //
    //     match variable.ty.as {
    //         Handle { .. } => {}
    //     }
    //     Ok(UboData {
    //         binding,
    //
    //     })
    // }
    pub fn reflect_ubos(
        vertex: GlobalVariable,
        fragment: GlobalVariable,
    ) -> Result<(), ShaderReflectError> {
        match (vertex.binding, fragment.binding) {
            // todo: should emit for both but whatever
            (None, None) | (Some(_), None) | (None, Some(_)) => {
                ShaderReflectError::VertexSemanticError(SemanticsErrorKind::InvalidDescriptorSet(
                    u32::MAX,
                ))
            }
            (Some(vert), Some(frag)) => {
                todo!();
            }
        };

        todo!();
        Ok(())
    }
}
#[cfg(test)]
mod test {
    use rspirv::dr::Instruction;
    use rspirv::spirv::Op;
    use librashader_preprocess::ShaderSource;

    #[test]
    pub fn test_into() {
        let result = ShaderSource::load("../test/slang-shaders/crt/shaders/crt-royale/src/crt-royale-scanlines-horizontal-apply-mask.slang").unwrap();
        let compilation = crate::front::GlslangCompilation::try_from(&result).unwrap();

        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_words(&compilation.vertex.as_binary(), &mut loader).unwrap();
        let module = loader.module();

        let outputs: Vec<&Instruction> = module.types_global_values.iter()
            .filter(|i| i.class.opcode == Op::Variable)
            .collect();

        println!("{:#?}", outputs);
    }
}
