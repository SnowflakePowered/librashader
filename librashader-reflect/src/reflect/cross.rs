
use crate::error::{ShaderReflectError, SemanticsErrorKind};
use crate::front::shaderc::GlslangCompilation;
use crate::reflect::semantics::{BindingStage, UboReflection, MAX_BINDINGS_COUNT, ShaderReflection, PushReflection, MAX_PUSH_BUFFER_SIZE, VariableSemantics, TextureSemantics};
use crate::reflect::{ReflectOptions, ReflectShader, UniformSemantic};
use spirv_cross::spirv::{Ast, Decoration, Module, Resource, ShaderResources, Type};
use std::fmt::Debug;
use spirv_cross::{ErrorCode, hlsl};
use spirv_cross::hlsl::{CompilerOptions, ShaderModel};

pub struct CrossReflect<T>
where
    T: spirv_cross::spirv::Target,
{
    vertex: Ast<T>,
    fragment: Ast<T>,
}

impl TryFrom<GlslangCompilation> for CrossReflect<hlsl::Target>
{
    type Error = ShaderReflectError;

    fn try_from(value: GlslangCompilation) -> Result<Self, Self::Error> {
        let vertex_module = Module::from_words(value.vertex.as_binary());
        let fragment_module = Module::from_words(value.fragment.as_binary());

        let mut vertex = Ast::parse(&vertex_module)?;
        let mut fragment = Ast::parse(&fragment_module)?;

        let mut options = CompilerOptions::default();
        options.shader_model = ShaderModel::V5_0;
        fragment.set_compiler_options(&options)?;
        vertex.set_compiler_options(&options)?;

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
                SemanticsErrorKind::InvalidUniformBufferCount(vertex_res.uniform_buffers.len()),
            ));
        }

        if vertex_res.push_constant_buffers.len() > 1 {
            return Err(ShaderReflectError::VertexSemanticError(
                SemanticsErrorKind::InvalidUniformBufferCount(vertex_res.push_constant_buffers.len()),
            ));
        }

        if fragment_res.uniform_buffers.len() > 1 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidUniformBufferCount(fragment_res.uniform_buffers.len()),
            ));
        }

        if fragment_res.push_constant_buffers.len() > 1 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidUniformBufferCount(fragment_res.push_constant_buffers.len()),
            ));
        }
        Ok(())
    }
}

#[derive(Copy, Clone)]
enum SemanticErrorBlame {
    Vertex,
    Fragment
}

struct UboData {
    id: u32,
    descriptor_set: u32,
    binding: u32,
    size: u32
}

impl SemanticErrorBlame {
    fn error(self, kind: SemanticsErrorKind) -> ShaderReflectError {
        return match self {
            SemanticErrorBlame::Vertex => ShaderReflectError::VertexSemanticError(kind),
            SemanticErrorBlame::Fragment => ShaderReflectError::FragmentSemanticError(kind)
        }
    }
}

impl <T> CrossReflect<T>
    where
        T: spirv_cross::spirv::Target,
        Ast<T>: spirv_cross::spirv::Compile<T>,
        Ast<T>: spirv_cross::spirv::Parse<T>,
{
    fn get_ubo_data(ast: &Ast<T>, ubo: &Resource, blame: SemanticErrorBlame) -> Result<UboData, ShaderReflectError> {
        let descriptor_set = ast.get_decoration(ubo.id, Decoration::DescriptorSet)?;
        let binding =  ast.get_decoration(ubo.id, Decoration::Binding)?;
        if binding  >= MAX_BINDINGS_COUNT {
            return Err(blame.error(SemanticsErrorKind::InvalidBinding(binding)))
        }
        if descriptor_set != 0 {
            return Err(blame.error(SemanticsErrorKind::InvalidDescriptorSet(descriptor_set)));
        }
        let size = ast.get_declared_struct_size(ubo.base_type_id)?;
        Ok(UboData {
            descriptor_set,
            binding,
            id: ubo.id,
            size
        })
    }

    fn get_push_size(ast: &Ast<T>, push: &Resource, blame: SemanticErrorBlame) -> Result<u32, ShaderReflectError> {
        let size = ast.get_declared_struct_size(push.base_type_id)?;
        if size >= MAX_PUSH_BUFFER_SIZE {
            return Err(blame.error(SemanticsErrorKind::InvalidPushBufferSize(size)));
        }
        Ok(size)
    }

    fn add_active_buffer_range(ast: &Ast<T>, resource: &Resource, options: &ReflectOptions, blame: SemanticErrorBlame) -> Result<(), ShaderReflectError> {
        let ranges = ast.get_active_buffer_ranges(resource.id)?;
        for range in ranges {
            let name = ast.get_member_name(resource.base_type_id, range.index)?;
            let res_type = ast.get_type(resource.base_type_id)?;
            let range_type = match res_type {
                Type::Struct { member_types, .. } => {
                    let range_type = member_types.get(range.index as usize)
                        .cloned()
                        .ok_or(blame.error(SemanticsErrorKind::InvalidRange(range.index)))?;
                    ast.get_type(range_type)?
                }
                _ => return Err(blame.error(SemanticsErrorKind::InvalidResourceType))
            };

            match options.uniform_semantics.get(&name) {
                None => return Err(blame.error(SemanticsErrorKind::UnknownSemantics(name))),
                Some(UniformSemantic::Variable(parameter)) => {
                    match &parameter.semantics {
                        VariableSemantics::FloatParameter => {}
                        semantics => {

                        }
                    }
                },
                Some(UniformSemantic::Texture(texture)) => {
                    if let TextureSemantics::PassOutput = texture.semantics {
                        if texture.index >= options.pass_number {
                            return Err(ShaderReflectError::NonCausalFilterChain { pass: options.pass_number, target: texture.index })
                        }
                    }

                    // todo: validaate type
                }
            }
        }
        Ok(())
    }

    fn reflect_ubos(&self, vertex_ubo: Option<&Resource>, fragment_ubo: Option<&Resource>) -> Result<Option<UboReflection>, ShaderReflectError> {
        match (vertex_ubo, fragment_ubo) {
            (None, None) => Ok(None),
            (Some(vertex_ubo), Some(fragment_ubo)) => {
                let vertex_ubo = Self::get_ubo_data(&self.vertex, vertex_ubo, SemanticErrorBlame::Vertex)?;
                let fragment_ubo = Self::get_ubo_data(&self.fragment, fragment_ubo, SemanticErrorBlame::Fragment)?;
                if vertex_ubo.binding != fragment_ubo.binding {
                    return Err(ShaderReflectError::MismatchedUniformBuffer {
                        vertex: vertex_ubo.binding,
                        fragment: fragment_ubo.binding
                    });
                }

                let size = std::cmp::max(vertex_ubo.size, fragment_ubo.size);
                Ok(Some(UboReflection {
                    binding: vertex_ubo.binding,
                    size,
                    stage_mask: BindingStage::VERTEX | BindingStage::FRAGMENT
                }))
            }
            (Some(vertex_ubo), None) => {
                let vertex_ubo = Self::get_ubo_data(&self.vertex, vertex_ubo, SemanticErrorBlame::Vertex)?;
                Ok(Some(UboReflection {
                    binding: vertex_ubo.binding,
                    size: vertex_ubo.size,
                    stage_mask: BindingStage::VERTEX
                }))
            }
            (None, Some(fragment_ubo)) => {
                let fragment_ubo = Self::get_ubo_data(&self.fragment, fragment_ubo, SemanticErrorBlame::Fragment)?;
                Ok(Some(UboReflection {
                    binding: fragment_ubo.binding,
                    size: fragment_ubo.size,
                    stage_mask: BindingStage::FRAGMENT
                }))
            }
        }
    }

    fn reflect_push_constant_buffer(&self, vertex_pcb: Option<&Resource>, fragment_pcb: Option<&Resource>) -> Result<Option<PushReflection>, ShaderReflectError> {
        match (vertex_pcb, fragment_pcb) {
            (None, None) => Ok(None),
            (Some(vertex_push), Some(fragment_push)) => {
                let vertex_size = Self::get_push_size(&self.vertex, vertex_push, SemanticErrorBlame::Vertex)?;
                let fragment_size = Self::get_push_size(&self.fragment, fragment_push, SemanticErrorBlame::Fragment)?;

                let size = std::cmp::max(vertex_size, fragment_size);

                Ok(Some(PushReflection {
                    size,
                    stage_mask: BindingStage::VERTEX | BindingStage::FRAGMENT
                }))
            }
            (Some(vertex_push), None) => {
                let vertex_size = Self::get_push_size(&self.vertex, vertex_push, SemanticErrorBlame::Vertex)?;
                Ok(Some(PushReflection {
                    size: vertex_size,
                    stage_mask: BindingStage::VERTEX
                }))
            }
            (None, Some(fragment_push)) => {
                let fragment_size = Self::get_push_size(&self.fragment, fragment_push, SemanticErrorBlame::Fragment)?;
                Ok(Some(PushReflection {
                    size: fragment_size,
                    stage_mask: BindingStage::FRAGMENT
                }))
            }
        }
    }
}


impl<T> ReflectShader for CrossReflect<T>
where
    T: spirv_cross::spirv::Target,
    Ast<T>: spirv_cross::spirv::Compile<T>,
    Ast<T>: spirv_cross::spirv::Parse<T>,
{
    fn reflect(&self, options: &ReflectOptions) -> Result<ShaderReflection, ShaderReflectError> {
        let vertex_res = self.vertex.get_shader_resources()?;
        let fragment_res = self.fragment.get_shader_resources()?;
        self.validate(&vertex_res, &fragment_res)?;

        let vertex_ubo = vertex_res.uniform_buffers.first().map(|f| f);
        let fragment_ubo = fragment_res.uniform_buffers.first().map(|f| f);

        let ubo = self.reflect_ubos(vertex_ubo, fragment_ubo)?;

        let vertex_push = vertex_res.push_constant_buffers.first().map(|f| f);
        let fragment_push = fragment_res.push_constant_buffers.first().map(|f| f);

        let push_constant = self.reflect_push_constant_buffer(vertex_push, fragment_push)?;

        Ok(ShaderReflection {
            ubo,
            push_constant
        })
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
        let mut reflect = CrossReflect::<hlsl::Target>::try_from(spirv).unwrap();
        eprintln!("{:#}", reflect.fragment.compile().unwrap())
        // let mut loader = rspirv::dr::Loader::new();
        // rspirv::binary::parse_words(spirv.fragment.as_binary(), &mut loader).unwrap();
        // let module = loader.module();
        // println!("{:#}", module.disassemble());
    }
}
