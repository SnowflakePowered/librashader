use crate::error::{SemanticsErrorKind, ShaderReflectError};
use crate::front::shaderc::GlslangCompilation;
use crate::reflect::semantics::{BindingStage, MemberOffset, PushReflection, SemanticMap, ShaderReflection, TextureSizeMeta, TextureSemantics, TypeInfo, UboReflection, ValidateTypeSemantics, VariableMeta, VariableSemantics, MAX_BINDINGS_COUNT, MAX_PUSH_BUFFER_SIZE, TextureImage};
use crate::reflect::{ReflectMeta, ReflectOptions, ReflectShader, UniformSemantic};
use rustc_hash::FxHashMap;
use spirv_cross::hlsl::{CompilerOptions, ShaderModel};
use spirv_cross::spirv::{Ast, Decoration, Module, Resource, ShaderResources, Type};
use spirv_cross::{hlsl, ErrorCode};
use std::fmt::Debug;
use std::str::FromStr;

pub struct CrossReflect<T>
where
    T: spirv_cross::spirv::Target,
{
    vertex: Ast<T>,
    fragment: Ast<T>,
}

impl ValidateTypeSemantics<Type> for VariableSemantics {
    fn validate_type(&self, ty: &Type) -> Option<TypeInfo> {
        let (&Type::Float { ref array, vecsize, columns } | &Type::Int { ref array, vecsize, columns } | &Type::UInt { ref array, vecsize, columns }) = ty else {
            return None
        };

        if !array.is_empty() {
            return None;
        }

        let valid = match self {
            VariableSemantics::MVP => {
                matches!(ty, &Type::Float { .. }) && vecsize == 4 && columns == 4
            }
            VariableSemantics::FrameCount => {
                matches!(ty, &Type::UInt { .. }) && vecsize == 1 && columns == 1
            }
            VariableSemantics::FrameDirection => {
                matches!(ty, &Type::Int { .. }) && vecsize == 1 && columns == 1
            }
            VariableSemantics::FloatParameter => {
                matches!(ty, &Type::Float { .. }) && vecsize == 1 && columns == 1
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
        let &Type::Float { ref array, vecsize, columns } = ty else {
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

impl TryFrom<GlslangCompilation> for CrossReflect<hlsl::Target> {
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

        let mut vert_mask = vertex_res.stage_inputs.iter().try_fold(0, |mask, input| {
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
        return match self {
            SemanticErrorBlame::Vertex => ShaderReflectError::VertexSemanticError(kind),
            SemanticErrorBlame::Fragment => ShaderReflectError::FragmentSemanticError(kind),
        };
    }
}

trait TextureSemanticMap<T> {
    fn get_texture_semantic(
        &self,
        name: &str,
    ) -> Option<SemanticMap<TextureSemantics>>;
}

trait VariableSemanticMap<T> {
    fn get_variable_semantic(
        &self,
        name: &str,
    ) -> Option<SemanticMap<VariableSemantics>>;
}

impl VariableSemanticMap<UniformSemantic> for FxHashMap<String, UniformSemantic> {
    fn get_variable_semantic(
        &self,
        name: &str,
    ) -> Option<SemanticMap<VariableSemantics>> {
        match self.get(name) {
            // existing uniforms in the semantic map have priority
            None => match name {
                "MVP" => Some(SemanticMap {
                    semantics: VariableSemantics::MVP,
                    index: 0,
                }),
                "OutputSize" => Some(SemanticMap {
                    semantics: VariableSemantics::Output,
                    index: 0,
                }),
                "FinalViewportSize" => Some(SemanticMap {
                    semantics: VariableSemantics::FinalViewport,
                    index: 0,
                }),
                "FrameCount" => Some(SemanticMap {
                    semantics: VariableSemantics::FrameCount,
                    index: 0,
                }),
                "FrameDirection" => Some(SemanticMap {
                    semantics: VariableSemantics::FrameDirection,
                    index: 0,
                }),
                _ => None
            },
            Some(UniformSemantic::Variable(variable)) => Some(*variable),
            Some(UniformSemantic::Texture(_)) => None,
        }
    }
}

impl TextureSemanticMap<UniformSemantic> for FxHashMap<String, UniformSemantic> {
    fn get_texture_semantic(
        &self,
        name: &str,
    ) -> Option<SemanticMap<TextureSemantics>> {
        match self.get(name) {
            None => {
                if let Some(semantics) = TextureSemantics::TEXTURE_SEMANTICS
                    .iter()
                    .find(|f| name.starts_with(f.size_uniform_name()))
                {
                    if semantics.is_array() {
                        let index = &name[semantics.size_uniform_name().len()..];
                        let Ok(index) = u32::from_str(index) else {
                            return None;
                        };
                       return Some(SemanticMap {
                            semantics: *semantics,
                            index,
                        });
                    } else if name == semantics.size_uniform_name() {
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index: 0,
                        });
                    }
                }
                return None
            }
            Some(UniformSemantic::Variable(_)) => None,
            Some(UniformSemantic::Texture(texture)) => Some(*texture),
        }
    }
}

impl TextureSemanticMap<UniformSemantic> for FxHashMap<String, SemanticMap<TextureSemantics>> {
    fn get_texture_semantic(
        &self,
        name: &str,
    ) -> Option<SemanticMap<TextureSemantics>> {
        match self.get(name) {
            None => {
                if let Some(semantics) = TextureSemantics::TEXTURE_SEMANTICS
                    .iter()
                    .find(|f| name.starts_with(f.texture_name()))
                {
                    if semantics.is_array() {
                        let index = &name[semantics.texture_name().len()..];
                        let Ok(index) = u32::from_str(index) else {return None};
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index,
                        });
                    } else if name == semantics.texture_name() {
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index: 0,
                        });
                    }
                }
                return None
            }
            Some(texture) => Some(*texture),
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
        options: &ReflectOptions,
        meta: &mut ReflectMeta,
        offset_type: impl Fn(usize) -> MemberOffset,
        blame: SemanticErrorBlame,
    ) -> Result<(), ShaderReflectError> {
        let ranges = ast.get_active_buffer_ranges(resource.id)?;
        eprintln!("{:?}", ranges);
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

            if let Some(parameter) = options
                .uniform_semantics
                .get_variable_semantic(&name)
            {
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
            } else if let Some(texture) = options
                .uniform_semantics
                .get_texture_semantic(&name)
            {
                let Some(_typeinfo) = texture.semantics.validate_type(&range_type) else {
                    return Err(blame.error(SemanticsErrorKind::InvalidTypeForSemantic(name)))
                };

                if let TextureSemantics::PassOutput = texture.semantics {
                    if texture.index >= options.pass_number {
                        return Err(ShaderReflectError::NonCausalFilterChain {
                            pass: options.pass_number,
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
                        SemanticErrorBlame::Fragment => BindingStage::FRAGMENT
                    });
                } else {
                    meta.texture_size_meta.insert(
                        texture.clone(),
                        TextureSizeMeta {
                            offset,
                            // todo: fix this.
                            stage_mask: match blame {
                                SemanticErrorBlame::Vertex => BindingStage::VERTEX,
                                SemanticErrorBlame::Fragment => BindingStage::FRAGMENT
                            }
                        },
                    );
                }
            } else {
                return Err(blame.error(SemanticsErrorKind::UnknownSemantics(name)))
            }
        }
        Ok(())
    }

    fn reflect_ubos(
        &self,
        vertex_ubo: Option<&Resource>,
        fragment_ubo: Option<&Resource>,
    ) -> Result<Option<UboReflection>, ShaderReflectError> {
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
        options: &ReflectOptions,
        meta: &mut ReflectMeta,
    ) -> Result<(), ShaderReflectError> {
        let Some(semantic) = options.non_uniform_semantics.get_texture_semantic(texture.name) else {
            return Err(SemanticErrorBlame::Fragment.error(SemanticsErrorKind::UnknownSemantics(texture.name.to_string())))
        };

        if semantic.semantics == TextureSemantics::PassOutput && semantic.index >= options.pass_number {
            return Err(ShaderReflectError::NonCausalFilterChain { pass: options.pass_number, target: semantic.index })
        }

        meta.texture_meta.insert(semantic, TextureImage {
            binding: texture.binding
        });
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
        &self,
        vertex_pcb: Option<&Resource>,
        fragment_pcb: Option<&Resource>,
    ) -> Result<Option<PushReflection>, ShaderReflectError> {
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

        let mut meta = ReflectMeta::default();

        if let Some(ubo) = vertex_ubo {
            Self::reflect_buffer_range_metas(
                &self.vertex,
                ubo,
                options,
                &mut meta,
                MemberOffset::Ubo,
                SemanticErrorBlame::Vertex,
            )?;
        }

        if let Some(ubo) = fragment_ubo {
            Self::reflect_buffer_range_metas(
                &self.fragment,
                ubo,
                options,
                &mut meta,
                MemberOffset::Ubo,
                SemanticErrorBlame::Fragment,
            )?;
        }

        if let Some(push) = vertex_push {
            Self::reflect_buffer_range_metas(
                &self.vertex,
                push,
                options,
                &mut meta,
                MemberOffset::PushConstant,
                SemanticErrorBlame::Vertex,
            )?;
        }

        if let Some(push) = fragment_push {
            Self::reflect_buffer_range_metas(
                &self.fragment,
                push,
                options,
                &mut meta,
                MemberOffset::PushConstant,
                SemanticErrorBlame::Fragment,
            )?;
        }

        // todo: slang_reflection: 556
        let mut ubo_bindings = 0u16;
        if vertex_ubo.is_some() || fragment_ubo.is_some() {
            ubo_bindings = 1 << ubo.as_ref().expect("UBOs should be present").binding;
        }

        for sampled_image in &fragment_res.sampled_images {
            let texture_data = self.reflect_texture(sampled_image)?;
            if ubo_bindings & (1 << texture_data.binding) != 0 {
                return Err(ShaderReflectError::BindingInUse(texture_data.binding));
            }
            ubo_bindings |= (1 << texture_data.binding);

            self.reflect_texture_metas(texture_data, options, &mut meta)?;
        }

        // slang-reflection:611


        Ok(ShaderReflection {
            ubo,
            push_constant,
            meta,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::reflect::cross::CrossReflect;
    use crate::reflect::{ReflectOptions, ReflectShader};
    use rspirv::binary::Disassemble;
    use spirv_cross::{glsl, hlsl};

    #[test]
    pub fn test_into() {
        let result = librashader_preprocess::load_shader_source("../test/basic.slang").unwrap();

        let spirv = crate::front::shaderc::compile_spirv(&result).unwrap();
        let mut reflect = CrossReflect::<hlsl::Target>::try_from(spirv).unwrap();
        let huh = reflect
            .reflect(&ReflectOptions {
                pass_number: 0,
                uniform_semantics: Default::default(),
                non_uniform_semantics: Default::default(),
            })
            .unwrap();
        eprintln!("{:#?}", huh);
        eprintln!("{:#}", reflect.fragment.compile().unwrap())
        // let mut loader = rspirv::dr::Loader::new();
        // rspirv::binary::parse_words(spirv.fragment.as_binary(), &mut loader).unwrap();
        // let module = loader.module();
        // println!("{:#}", module.disassemble());
    }
}
