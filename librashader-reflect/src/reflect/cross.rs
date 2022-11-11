use crate::error::{SemanticsErrorKind, ShaderCompileError, ShaderReflectError};
use crate::front::shaderc::GlslangCompilation;
use crate::reflect::semantics::{
    BindingStage, MemberOffset, PushReflection, SemanticMap, ShaderReflection, TextureImage,
    TextureSemantics, TextureSizeMeta, TypeInfo, UboReflection, ValidateTypeSemantics,
    VariableMeta, VariableSemantics, MAX_BINDINGS_COUNT, MAX_PUSH_BUFFER_SIZE,
};
use crate::reflect::{
    ReflectMeta, ReflectSemantics, ReflectShader, TextureSemanticMap, UniformSemantic,
    VariableSemanticMap,
};
use rustc_hash::FxHashMap;
use spirv_cross::hlsl::ShaderModel;
use spirv_cross::spirv::{Ast, Decoration, Module, Resource, ShaderResources, Type};
use spirv_cross::{glsl, hlsl, ErrorCode};

use crate::back::targets::{GLSL, HLSL};
use crate::back::{CompileShader, CompiledShader};
use std::str::FromStr;

pub struct CrossReflect<T>
where
    T: spirv_cross::spirv::Target,
{
    vertex: Ast<T>,
    fragment: Ast<T>,
}

pub(crate) type HlslReflect = CrossReflect<hlsl::Target>;
pub(crate) type GlslReflect = CrossReflect<glsl::Target>;

impl ValidateTypeSemantics<Type> for VariableSemantics {
    fn validate_type(&self, ty: &Type) -> Option<TypeInfo> {
        let (Type::Float { ref array, vecsize, columns } | Type::Int { ref array, vecsize, columns } | Type::UInt { ref array, vecsize, columns }) = *ty else {
            return None
        };

        if !array.is_empty() {
            return None;
        }

        let valid = match self {
            VariableSemantics::MVP => {
                matches!(ty, Type::Float { .. }) && vecsize == 4 && columns == 4
            }
            VariableSemantics::FrameCount => {
                matches!(ty, Type::UInt { .. }) && vecsize == 1 && columns == 1
            }
            VariableSemantics::FrameDirection => {
                matches!(ty, Type::Int { .. }) && vecsize == 1 && columns == 1
            }
            VariableSemantics::FloatParameter => {
                matches!(ty, Type::Float { .. }) && vecsize == 1 && columns == 1
            }
            _ => matches!(ty, Type::Float { .. }) && vecsize == 4 && columns == 1,
        };

        if valid {
            Some(TypeInfo {
                size: vecsize,
                columns,
            })
        } else {
            None
        }
    }
}

impl ValidateTypeSemantics<Type> for TextureSemantics {
    fn validate_type(&self, ty: &Type) -> Option<TypeInfo> {
        let Type::Float { ref array, vecsize, columns } = *ty else {
            return None
        };

        if !array.is_empty() {
            return None;
        }

        if vecsize == 4 && columns == 1 {
            Some(TypeInfo {
                size: vecsize,
                columns,
            })
        } else {
            None
        }
    }
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

        let mut vertex = Ast::parse(&vertex_module)?;
        let mut fragment = Ast::parse(&fragment_module)?;

        Ok(CrossReflect { vertex, fragment })
    }
}

impl<T> CrossReflect<T>
where
    T: spirv_cross::spirv::Target,
    Ast<T>: spirv_cross::spirv::Compile<T>,
    Ast<T>: spirv_cross::spirv::Parse<T>,
{
    fn validate(
        &self,
        vertex_res: &ShaderResources,
        fragment_res: &ShaderResources,
    ) -> Result<(), ShaderReflectError> {
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

        let fragment_location = self
            .fragment
            .get_decoration(fragment_res.stage_outputs[0].id, Decoration::Location)?;
        if fragment_location != 0 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidLocation(fragment_location),
            ));
        }

        let vert_mask = vertex_res.stage_inputs.iter().try_fold(0, |mask, input| {
            Ok::<u32, ErrorCode>(
                mask | 1 << self.vertex.get_decoration(input.id, Decoration::Location)?,
            )
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
                SemanticsErrorKind::InvalidUniformBufferCount(
                    vertex_res.push_constant_buffers.len(),
                ),
            ));
        }

        if fragment_res.uniform_buffers.len() > 1 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidUniformBufferCount(fragment_res.uniform_buffers.len()),
            ));
        }

        if fragment_res.push_constant_buffers.len() > 1 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidUniformBufferCount(
                    fragment_res.push_constant_buffers.len(),
                ),
            ));
        }
        Ok(())
    }
}

struct UboData {
    id: u32,
    descriptor_set: u32,
    binding: u32,
    size: u32,
}

struct TextureData<'a> {
    id: u32,
    name: &'a str,
    descriptor_set: u32,
    binding: u32,
}

// todo: might want to take these crate helpers out.

#[derive(Copy, Clone)]
enum SemanticErrorBlame {
    Vertex,
    Fragment,
}

impl SemanticErrorBlame {
    fn error(self, kind: SemanticsErrorKind) -> ShaderReflectError {
        match self {
            SemanticErrorBlame::Vertex => ShaderReflectError::VertexSemanticError(kind),
            SemanticErrorBlame::Fragment => ShaderReflectError::FragmentSemanticError(kind),
        }
    }
}

impl<T> CrossReflect<T>
where
    T: spirv_cross::spirv::Target,
    Ast<T>: spirv_cross::spirv::Compile<T>,
    Ast<T>: spirv_cross::spirv::Parse<T>,
{
    fn get_ubo_data(
        ast: &Ast<T>,
        ubo: &Resource,
        blame: SemanticErrorBlame,
    ) -> Result<UboData, ShaderReflectError> {
        let descriptor_set = ast.get_decoration(ubo.id, Decoration::DescriptorSet)?;
        let binding = ast.get_decoration(ubo.id, Decoration::Binding)?;
        if binding >= MAX_BINDINGS_COUNT {
            return Err(blame.error(SemanticsErrorKind::InvalidBinding(binding)));
        }
        if descriptor_set != 0 {
            return Err(blame.error(SemanticsErrorKind::InvalidDescriptorSet(descriptor_set)));
        }
        let size = ast.get_declared_struct_size(ubo.base_type_id)?;
        Ok(UboData {
            descriptor_set,
            binding,
            id: ubo.id,
            size,
        })
    }

    fn get_push_size(
        ast: &Ast<T>,
        push: &Resource,
        blame: SemanticErrorBlame,
    ) -> Result<u32, ShaderReflectError> {
        let size = ast.get_declared_struct_size(push.base_type_id)?;
        if size >= MAX_PUSH_BUFFER_SIZE {
            return Err(blame.error(SemanticsErrorKind::InvalidPushBufferSize(size)));
        }
        Ok(size)
    }

    fn reflect_buffer_range_metas(
        ast: &Ast<T>,
        resource: &Resource,
        pass_number: u32,
        semantics: &ReflectSemantics,
        meta: &mut ReflectMeta,
        offset_type: impl Fn(usize) -> MemberOffset,
        blame: SemanticErrorBlame,
    ) -> Result<(), ShaderReflectError> {
        let ranges = ast.get_active_buffer_ranges(resource.id)?;
        eprintln!("{ranges:?}");
        for range in ranges {
            let name = ast.get_member_name(resource.base_type_id, range.index)?;
            let ubo_type = ast.get_type(resource.base_type_id)?;
            let range_type = match ubo_type {
                Type::Struct { member_types, .. } => {
                    let range_type = member_types
                        .get(range.index as usize)
                        .cloned()
                        .ok_or(blame.error(SemanticsErrorKind::InvalidRange(range.index)))?;
                    ast.get_type(range_type)?
                }
                _ => return Err(blame.error(SemanticsErrorKind::InvalidResourceType)),
            };

            if let Some(parameter) = semantics.uniform_semantics.get_variable_semantic(&name) {
                let Some(typeinfo) = parameter.semantics.validate_type(&range_type) else {
                    return Err(blame.error(SemanticsErrorKind::InvalidTypeForSemantic(name)))
                };

                match &parameter.semantics {
                    VariableSemantics::FloatParameter => {
                        let offset = offset_type(range.offset);
                        if let Some(meta) = meta.parameter_meta.get(&parameter.index) {
                            if offset != meta.offset {
                                return Err(ShaderReflectError::MismatchedOffset {
                                    semantic: name,
                                    vertex: meta.offset,
                                    fragment: offset,
                                });
                            }
                            if meta.components != typeinfo.size {
                                return Err(ShaderReflectError::MismatchedComponent {
                                    semantic: name,
                                    vertex: meta.components,
                                    fragment: typeinfo.size,
                                });
                            }
                        } else {
                            meta.parameter_meta.insert(
                                parameter.index,
                                VariableMeta {
                                    offset,
                                    components: typeinfo.size,
                                },
                            );
                        }
                    }
                    semantics => {
                        let offset = offset_type(range.offset);
                        if let Some(meta) = meta.variable_meta.get(semantics) {
                            if offset != meta.offset {
                                return Err(ShaderReflectError::MismatchedOffset {
                                    semantic: name,
                                    vertex: meta.offset,
                                    fragment: offset,
                                });
                            }
                            if meta.components != typeinfo.size * typeinfo.columns {
                                return Err(ShaderReflectError::MismatchedComponent {
                                    semantic: name,
                                    vertex: meta.components,
                                    fragment: typeinfo.size,
                                });
                            }
                        } else {
                            meta.variable_meta.insert(
                                *semantics,
                                VariableMeta {
                                    offset,
                                    components: typeinfo.size * typeinfo.columns,
                                },
                            );
                        }
                    }
                }
            } else if let Some(texture) = semantics.uniform_semantics.get_texture_semantic(&name) {
                let Some(_typeinfo) = texture.semantics.validate_type(&range_type) else {
                    return Err(blame.error(SemanticsErrorKind::InvalidTypeForSemantic(name)))
                };

                if let TextureSemantics::PassOutput = texture.semantics {
                    if texture.index >= pass_number {
                        return Err(ShaderReflectError::NonCausalFilterChain {
                            pass: pass_number,
                            target: texture.index,
                        });
                    }
                }

                let offset = offset_type(range.offset);
                if let Some(meta) = meta.texture_size_meta.get_mut(&texture) {
                    if offset != meta.offset {
                        return Err(ShaderReflectError::MismatchedOffset {
                            semantic: name,
                            vertex: meta.offset,
                            fragment: offset,
                        });
                    }

                    meta.stage_mask.insert(match blame {
                        SemanticErrorBlame::Vertex => BindingStage::VERTEX,
                        SemanticErrorBlame::Fragment => BindingStage::FRAGMENT,
                    });
                } else {
                    meta.texture_size_meta.insert(
                        texture,
                        TextureSizeMeta {
                            offset,
                            // todo: fix this. to allow both
                            stage_mask: match blame {
                                SemanticErrorBlame::Vertex => BindingStage::VERTEX,
                                SemanticErrorBlame::Fragment => BindingStage::FRAGMENT,
                            },
                        },
                    );
                }
            } else {
                return Err(blame.error(SemanticsErrorKind::UnknownSemantics(name)));
            }
        }
        Ok(())
    }

    fn reflect_ubos(
        &mut self,
        vertex_ubo: Option<&Resource>,
        fragment_ubo: Option<&Resource>,
    ) -> Result<Option<UboReflection>, ShaderReflectError> {
        if let Some(vertex_ubo) = vertex_ubo {
            self.vertex
                .set_decoration(vertex_ubo.id, Decoration::Binding, 0)?;
        }

        if let Some(fragment_ubo) = fragment_ubo {
            self.fragment
                .set_decoration(fragment_ubo.id, Decoration::Binding, 0)?;
        }

        match (vertex_ubo, fragment_ubo) {
            (None, None) => Ok(None),
            (Some(vertex_ubo), Some(fragment_ubo)) => {
                let vertex_ubo =
                    Self::get_ubo_data(&self.vertex, vertex_ubo, SemanticErrorBlame::Vertex)?;
                let fragment_ubo =
                    Self::get_ubo_data(&self.fragment, fragment_ubo, SemanticErrorBlame::Fragment)?;
                if vertex_ubo.binding != fragment_ubo.binding {
                    return Err(ShaderReflectError::MismatchedUniformBuffer {
                        vertex: vertex_ubo.binding,
                        fragment: fragment_ubo.binding,
                    });
                }

                let size = std::cmp::max(vertex_ubo.size, fragment_ubo.size);
                Ok(Some(UboReflection {
                    binding: vertex_ubo.binding,
                    size,
                    stage_mask: BindingStage::VERTEX | BindingStage::FRAGMENT,
                }))
            }
            (Some(vertex_ubo), None) => {
                let vertex_ubo =
                    Self::get_ubo_data(&self.vertex, vertex_ubo, SemanticErrorBlame::Vertex)?;
                Ok(Some(UboReflection {
                    binding: vertex_ubo.binding,
                    size: vertex_ubo.size,
                    stage_mask: BindingStage::VERTEX,
                }))
            }
            (None, Some(fragment_ubo)) => {
                let fragment_ubo =
                    Self::get_ubo_data(&self.fragment, fragment_ubo, SemanticErrorBlame::Fragment)?;
                Ok(Some(UboReflection {
                    binding: fragment_ubo.binding,
                    size: fragment_ubo.size,
                    stage_mask: BindingStage::FRAGMENT,
                }))
            }
        }
    }

    fn reflect_texture_metas(
        &self,
        texture: TextureData,
        pass_number: u32,
        semantics: &ReflectSemantics,
        meta: &mut ReflectMeta,
    ) -> Result<(), ShaderReflectError> {
        let Some(semantic) = semantics.non_uniform_semantics.get_texture_semantic(texture.name) else {
            return Err(SemanticErrorBlame::Fragment.error(SemanticsErrorKind::UnknownSemantics(texture.name.to_string())))
        };

        if semantic.semantics == TextureSemantics::PassOutput && semantic.index >= pass_number {
            return Err(ShaderReflectError::NonCausalFilterChain {
                pass: pass_number,
                target: semantic.index,
            });
        }

        meta.texture_meta.insert(
            semantic,
            TextureImage {
                binding: texture.binding,
            },
        );
        Ok(())
    }

    fn reflect_texture<'a>(
        &'a self,
        texture: &'a Resource,
    ) -> Result<TextureData<'a>, ShaderReflectError> {
        let descriptor_set = self
            .fragment
            .get_decoration(texture.id, Decoration::DescriptorSet)?;
        let binding = self
            .fragment
            .get_decoration(texture.id, Decoration::Binding)?;
        if descriptor_set != 0 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidDescriptorSet(descriptor_set),
            ));
        }
        if binding >= MAX_BINDINGS_COUNT {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidBinding(binding),
            ));
        }
        Ok(TextureData {
            id: texture.id,
            name: &texture.name,
            descriptor_set,
            binding,
        })
    }

    fn reflect_push_constant_buffer(
        &mut self,
        vertex_pcb: Option<&Resource>,
        fragment_pcb: Option<&Resource>,
    ) -> Result<Option<PushReflection>, ShaderReflectError> {
        if let Some(vertex_pcb) = vertex_pcb {
            self.vertex
                .set_decoration(vertex_pcb.id, Decoration::Binding, 1)?;
        }

        if let Some(fragment_pcb) = fragment_pcb {
            self.fragment
                .set_decoration(fragment_pcb.id, Decoration::Binding, 1)?;
        }

        match (vertex_pcb, fragment_pcb) {
            (None, None) => Ok(None),
            (Some(vertex_push), Some(fragment_push)) => {
                let vertex_size =
                    Self::get_push_size(&self.vertex, vertex_push, SemanticErrorBlame::Vertex)?;
                let fragment_size = Self::get_push_size(
                    &self.fragment,
                    fragment_push,
                    SemanticErrorBlame::Fragment,
                )?;

                let size = std::cmp::max(vertex_size, fragment_size);

                Ok(Some(PushReflection {
                    size,
                    stage_mask: BindingStage::VERTEX | BindingStage::FRAGMENT,
                }))
            }
            (Some(vertex_push), None) => {
                let vertex_size =
                    Self::get_push_size(&self.vertex, vertex_push, SemanticErrorBlame::Vertex)?;
                Ok(Some(PushReflection {
                    size: vertex_size,
                    stage_mask: BindingStage::VERTEX,
                }))
            }
            (None, Some(fragment_push)) => {
                let fragment_size = Self::get_push_size(
                    &self.fragment,
                    fragment_push,
                    SemanticErrorBlame::Fragment,
                )?;
                Ok(Some(PushReflection {
                    size: fragment_size,
                    stage_mask: BindingStage::FRAGMENT,
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
    fn reflect(
        &mut self,
        pass_number: u32,
        semantics: &ReflectSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError> {
        let vertex_res = self.vertex.get_shader_resources()?;
        let fragment_res = self.fragment.get_shader_resources()?;
        self.validate(&vertex_res, &fragment_res)?;

        let vertex_ubo = vertex_res.uniform_buffers.first();
        let fragment_ubo = fragment_res.uniform_buffers.first();

        let ubo = self.reflect_ubos(vertex_ubo, fragment_ubo)?;

        let vertex_push = vertex_res.push_constant_buffers.first();
        let fragment_push = fragment_res.push_constant_buffers.first();

        let push_constant = self.reflect_push_constant_buffer(vertex_push, fragment_push)?;

        let mut meta = ReflectMeta::default();

        if let Some(ubo) = vertex_ubo {
            Self::reflect_buffer_range_metas(
                &self.vertex,
                ubo,
                pass_number,
                semantics,
                &mut meta,
                MemberOffset::Ubo,
                SemanticErrorBlame::Vertex,
            )?;
        }

        if let Some(ubo) = fragment_ubo {
            Self::reflect_buffer_range_metas(
                &self.fragment,
                ubo,
                pass_number,
                semantics,
                &mut meta,
                MemberOffset::Ubo,
                SemanticErrorBlame::Fragment,
            )?;
        }

        if let Some(push) = vertex_push {
            Self::reflect_buffer_range_metas(
                &self.vertex,
                push,
                pass_number,
                semantics,
                &mut meta,
                MemberOffset::PushConstant,
                SemanticErrorBlame::Vertex,
            )?;
        }

        if let Some(push) = fragment_push {
            Self::reflect_buffer_range_metas(
                &self.fragment,
                push,
                pass_number,
                semantics,
                &mut meta,
                MemberOffset::PushConstant,
                SemanticErrorBlame::Fragment,
            )?;
        }

        let mut ubo_bindings = 0u16;
        if vertex_ubo.is_some() || fragment_ubo.is_some() {
            ubo_bindings = 1 << ubo.as_ref().expect("UBOs should be present").binding;
        }

        for sampled_image in &fragment_res.sampled_images {
            let texture_data = self.reflect_texture(sampled_image)?;
            if ubo_bindings & (1 << texture_data.binding) != 0 {
                return Err(ShaderReflectError::BindingInUse(texture_data.binding));
            }
            ubo_bindings |= 1 << texture_data.binding;

            self.reflect_texture_metas(texture_data, pass_number, semantics, &mut meta)?;
        }

        Ok(ShaderReflection {
            ubo,
            push_constant,
            meta,
        })
    }
}

impl CompileShader<GLSL> for CrossReflect<glsl::Target> {
    type Options = glsl::Version;

    // todo: compile should consume self
    fn compile(
        &mut self,
        version: Self::Options,
    ) -> Result<CompiledShader<String, Vec<u32>>, ShaderCompileError> {
        let mut options: glsl::CompilerOptions = Default::default();
        options.version = version;
        options.fragment.default_float_precision = glsl::Precision::High;
        options.fragment.default_int_precision = glsl::Precision::High;
        options.enable_420_pack_extension = false;

        self.vertex.set_compiler_options(&options)?;
        self.fragment.set_compiler_options(&options)?;

        let vertex_resources = self.vertex.get_shader_resources()?;
        let fragment_resources = self.fragment.get_shader_resources()?;

        for res in &vertex_resources.stage_inputs {
            let location = self.vertex.get_decoration(res.id, Decoration::Location)?;
            self.vertex
                .set_name(res.id, &format!("RARCH_ATTRIBUTE_{location}"))?;
            self.vertex.unset_decoration(res.id, Decoration::Location)?;
        }
        for res in &vertex_resources.stage_outputs {
            let location = self.vertex.get_decoration(res.id, Decoration::Location)?;
            self.vertex
                .set_name(res.id, &format!("RARCH_VARYING_{location}"))?;
            self.vertex.unset_decoration(res.id, Decoration::Location)?;
        }
        for res in &fragment_resources.stage_inputs {
            let location = self.fragment.get_decoration(res.id, Decoration::Location)?;
            self.fragment
                .set_name(res.id, &format!("RARCH_VARYING_{location}"))?;
            self.fragment
                .unset_decoration(res.id, Decoration::Location)?;
        }

        if vertex_resources.push_constant_buffers.len() > 1 {
            return Err(ShaderCompileError::SpirvCrossCompileError(
                ErrorCode::CompilationError(String::from(
                    "Cannot have more than one push constant buffer",
                )),
            ));
        }
        for res in &vertex_resources.push_constant_buffers {
            self.vertex.set_name(res.id, "RARCH_PUSH_VERTEX_INSTANCE")?;
            self.vertex
                .set_name(res.base_type_id, "RARCH_PUSH_VERTEX")?;
        }

        // todo: options
        let flatten = false;

        if vertex_resources.uniform_buffers.len() > 1 {
            return Err(ShaderCompileError::SpirvCrossCompileError(
                ErrorCode::CompilationError(String::from(
                    "Cannot have more than one uniform buffer",
                )),
            ));
        }
        for res in &vertex_resources.uniform_buffers {
            if flatten {
                self.vertex.flatten_buffer_block(res.id)?;
            }
            self.vertex.set_name(res.id, "RARCH_UBO_VERTEX_INSTANCE")?;
            self.vertex.set_name(res.base_type_id, "RARCH_UBO_VERTEX")?;
            self.vertex
                .unset_decoration(res.id, Decoration::DescriptorSet)?;
            self.vertex.unset_decoration(res.id, Decoration::Binding)?;
        }

        if fragment_resources.push_constant_buffers.len() > 1 {
            return Err(ShaderCompileError::SpirvCrossCompileError(
                ErrorCode::CompilationError(String::from(
                    "Cannot have more than one push constant buffer",
                )),
            ));
        }
        for res in &fragment_resources.push_constant_buffers {
            self.fragment
                .set_name(res.id, "RARCH_PUSH_FRAGMENT_INSTANCE")?;
            self.fragment
                .set_name(res.base_type_id, "RARCH_PUSH_FRAGMENT")?;
        }

        if fragment_resources.uniform_buffers.len() > 1 {
            return Err(ShaderCompileError::SpirvCrossCompileError(
                ErrorCode::CompilationError(String::from(
                    "Cannot have more than one uniform buffer",
                )),
            ));
        }

        for res in &fragment_resources.uniform_buffers {
            if flatten {
                self.fragment.flatten_buffer_block(res.id)?;
            }
            self.fragment
                .set_name(res.id, "RARCH_UBO_FRAGMENT_INSTANCE")?;
            self.fragment
                .set_name(res.base_type_id, "RARCH_UBO_FRAGMENT")?;
            self.fragment
                .unset_decoration(res.id, Decoration::DescriptorSet)?;
            self.fragment
                .unset_decoration(res.id, Decoration::Binding)?;
        }

        let mut texture_fixups = Vec::new();
        for res in &fragment_resources.sampled_images {
            let binding = self.fragment.get_decoration(res.id, Decoration::Binding)?;
            self.fragment
                .set_name(res.id, &format!("RARCH_TEXTURE_{binding}"))?;
            self.fragment
                .unset_decoration(res.id, Decoration::DescriptorSet)?;
            self.fragment
                .unset_decoration(res.id, Decoration::Binding)?;
            texture_fixups.push(binding);
        }

        Ok(CompiledShader {
            vertex: self.vertex.compile()?,
            fragment: self.fragment.compile()?,
            context: texture_fixups,
        })
    }
}

impl CompileShader<HLSL> for CrossReflect<hlsl::Target> {
    type Options = Option<()>;

    fn compile(
        &mut self,
        _options: Self::Options,
    ) -> Result<CompiledShader<String>, ShaderCompileError> {
        let mut options = hlsl::CompilerOptions::default();
        options.shader_model = ShaderModel::V5_0;

        self.vertex.set_compiler_options(&options)?;
        self.fragment.set_compiler_options(&options)?;

        Ok(CompiledShader {
            vertex: self.vertex.compile()?,
            fragment: self.fragment.compile()?,
            context: (),
        })
    }
}

#[cfg(test)]
mod test {
    use crate::reflect::cross::CrossReflect;
    use crate::reflect::{ReflectSemantics, ReflectShader, UniformSemantic};
    use rustc_hash::FxHashMap;

    use crate::back::CompileShader;
    use crate::reflect::semantics::{SemanticMap, VariableSemantics};
    use spirv_cross::glsl::{CompilerOptions, Version};
    use spirv_cross::{glsl, hlsl};

    #[test]
    pub fn test_into() {
        let result = librashader_preprocess::load_shader_source("../test/basic.slang").unwrap();
        let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();

        for (index, param) in result.parameters.iter().enumerate() {
            uniform_semantics.insert(
                param.id.clone(),
                UniformSemantic::Variable(SemanticMap {
                    semantics: VariableSemantics::FloatParameter,
                    index: index as u32,
                }),
            );
        }
        let spirv = crate::front::shaderc::compile_spirv(&result).unwrap();
        let mut reflect = CrossReflect::<glsl::Target>::try_from(spirv).unwrap();
        let shader_reflection = reflect
            .reflect(
                0,
                &ReflectSemantics {
                    uniform_semantics,
                    non_uniform_semantics: Default::default(),
                },
            )
            .unwrap();
        let mut opts = CompilerOptions::default();
        opts.version = Version::V4_60;
        opts.enable_420_pack_extension = false;
        let compiled = reflect.compile(&opts, &shader_reflection).unwrap();
        // eprintln!("{shader_reflection:#?}");
        eprintln!("{:#}", compiled.fragment)
        // let mut loader = rspirv::dr::Loader::new();
        // rspirv::binary::parse_words(spirv.fragment.as_binary(), &mut loader).unwrap();
        // let module = loader.module();
        // println!("{:#}", module.disassemble());
    }
}
