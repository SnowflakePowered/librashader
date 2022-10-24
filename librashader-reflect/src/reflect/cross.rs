use crate::error::{ShaderReflectError, SemanticsErrorKind};
use crate::front::shaderc::GlslangCompilation;
use crate::reflect::semantics::{MAX_BINDING_NUM, MAX_BINDINGS_COUNT, ShaderReflection};
use crate::reflect::ReflectShader;
use spirv_cross::spirv::{Ast, Decoration, Module, ShaderResources};
use std::fmt::Debug;
use spirv_cross::ErrorCode;

pub struct CrossReflect<T>
where
    T: spirv_cross::spirv::Target,
{
    vertex: Ast<T>,
    fragment: Ast<T>,
}

impl<T> TryFrom<GlslangCompilation> for CrossReflect<T>
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

        Ok(CrossReflect { vertex, fragment })
    }
}
impl <T> CrossReflect<T>
    where
        T: spirv_cross::spirv::Target,
        Ast<T>: spirv_cross::spirv::Compile<T>,
        Ast<T>: spirv_cross::spirv::Parse<T>,
{
    fn validate(&self, vertex_res: &ShaderResources, fragment_res: &ShaderResources) -> Result<(), ShaderReflectError> {
        if !vertex_res.sampled_images.is_empty()
            || !vertex_res.storage_buffers.is_empty()
            || !vertex_res.subpass_inputs.is_empty()
            || !vertex_res.storage_images.is_empty()
            || !vertex_res.atomic_counters.is_empty()
        {
            return Err(ShaderReflectError::VertexSemanticError(
                SemanticsErrorKind::InvalidResourceType,
            ));
        }

        if !fragment_res.storage_buffers.is_empty()
            || !fragment_res.subpass_inputs.is_empty()
            || !fragment_res.storage_images.is_empty()
            || !fragment_res.atomic_counters.is_empty()
        {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidResourceType,
            ));
        }

        let vert_inputs = vertex_res.stage_inputs.len();
        if vert_inputs != 2 {
            return Err(ShaderReflectError::VertexSemanticError(
                SemanticsErrorKind::InvalidInputCount(vert_inputs),
            ));
        }

        let frag_outputs = fragment_res.stage_outputs.len();
        if frag_outputs != 1 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidOutputCount(frag_outputs),
            ));
        }

        let fragment_location = self.fragment.get_decoration(fragment_res.stage_outputs[0].id, Decoration::Location)?;
        if fragment_location != 0 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidLocation(fragment_location),
            ));
        }

        let mut vert_mask = vertex_res.stage_inputs.iter()
            .try_fold(0, |mask, input| {
                Ok::<u32, ErrorCode>(mask | 1 << self.vertex.get_decoration(input.id, Decoration::Location)?)
            })?;
        if vert_mask != 0x3 {
            return Err(ShaderReflectError::VertexSemanticError(
                SemanticsErrorKind::InvalidLocation(vert_mask),
            ));
        }

        if vertex_res.uniform_buffers.len() > 1 {
            return Err(ShaderReflectError::VertexSemanticError(
                SemanticsErrorKind::InvalidUniformBufferSize(vertex_res.uniform_buffers.len()),
            ));
        }

        if vertex_res.push_constant_buffers.len() > 1 {
            return Err(ShaderReflectError::VertexSemanticError(
                SemanticsErrorKind::InvalidUniformBufferSize(vertex_res.push_constant_buffers.len()),
            ));
        }

        if fragment_res.uniform_buffers.len() > 1 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidUniformBufferSize(fragment_res.uniform_buffers.len()),
            ));
        }

        if fragment_res.push_constant_buffers.len() > 1 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidUniformBufferSize(fragment_res.push_constant_buffers.len()),
            ));
        }
        Ok(())
    }
}

impl<T> ReflectShader for CrossReflect<T>
where
    T: spirv_cross::spirv::Target,
    Ast<T>: spirv_cross::spirv::Compile<T>,
    Ast<T>: spirv_cross::spirv::Parse<T>,
{
    fn reflect(&self) -> Result<ShaderReflection, ShaderReflectError> {
        let vertex_res = self.vertex.get_shader_resources()?;
        let fragment_res = self.fragment.get_shader_resources()?;
        self.validate(&vertex_res, &fragment_res)?;

        let vertex_ubo = vertex_res.uniform_buffers.first().map(|f| f.id);
        let fragment_ubo = fragment_res.uniform_buffers.first().map(|f| f.id);

        let vertex_push = vertex_res.push_constant_buffers.first().map(|f| f.id);
        let fragment_push = fragment_res.push_constant_buffers.first().map(|f| f.id);

        if let Some(ubo) = vertex_ubo {
            let desc_set = self.vertex.get_decoration(ubo, Decoration::DescriptorSet)?;
            if desc_set != 0 {
                return Err(ShaderReflectError::VertexSemanticError(SemanticsErrorKind::InvalidDescriptorSet(desc_set)))
            }
        }

        if let Some(ubo) = fragment_ubo {
            let desc_set = self.fragment.get_decoration(ubo, Decoration::DescriptorSet)?;
            if desc_set != 0 {
                return Err(ShaderReflectError::FragmentSemanticError(SemanticsErrorKind::InvalidDescriptorSet(desc_set)))
            }
        }

        let vertex_ubo_binding = vertex_ubo.map(|s| self.vertex.get_decoration(s, Decoration::Binding))
            .map_or(Ok(None), |v| v.map(Some))?;

        let fragment_ubo_binding = vertex_ubo.map(|s| self.fragment.get_decoration(s, Decoration::Binding))
            .map_or(Ok(None), |v| v.map(Some))?;

        match (vertex_ubo_binding, fragment_ubo_binding) {
            (Some(vertex), Some(fragment)) => {
                if vertex != fragment {
                    return Err(ShaderReflectError::MismatchedUniformBuffer {
                        vertex: vertex_ubo_binding,
                        fragment: fragment_ubo_binding
                    })
                }

                if vertex >= MAX_BINDINGS_COUNT {
                    return Err(ShaderReflectError::InvalidBinding(vertex))
                }
            }
            (Some(vertex), None) => {
                if vertex >= MAX_BINDINGS_COUNT {
                    return Err(ShaderReflectError::VertexSemanticError(SemanticsErrorKind::InvalidBinding(vertex)));
                }
            }
            (None, Some(fragment)) => {
                if fragment >= MAX_BINDINGS_COUNT {
                    return Err(ShaderReflectError::FragmentSemanticError(SemanticsErrorKind::InvalidBinding(vertex)));
                }
            }
            (None, None) => {}
        }

        // todo: slang_reflection:490
        todo!()
    }
}

#[cfg(test)]
mod test {
    use crate::reflect::cross::CrossReflect;
    use rspirv::binary::Disassemble;
    use spirv_cross::{glsl, hlsl};

    #[test]
    pub fn test_into() {
        let result = librashader_preprocess::load_shader_source("../test/basic.slang").unwrap();
        let spirv = crate::front::shaderc::compile_spirv(&result).unwrap();
        let mut reflect = CrossReflect::<glsl::Target>::try_from(spirv).unwrap();
        // let mut loader = rspirv::dr::Loader::new();
        // rspirv::binary::parse_words(spirv.fragment.as_binary(), &mut loader).unwrap();
        // let module = loader.module();
        // println!("{:#}", module.disassemble());
    }
}
