use crate::error::{SemanticsErrorKind, ShaderReflectError};

use naga::{
    AddressSpace, Binding, GlobalVariable, Handle, ImageClass, Module, ResourceBinding, ScalarKind,
    TypeInner, VectorSize,
};

use crate::reflect::helper::{SemanticErrorBlame, TextureData, UboData};
use crate::reflect::semantics::{
    BindingMeta, BindingStage, MemberOffset, PushReflection, ShaderSemantics, TextureBinding,
    TextureSemanticMap, TextureSemantics, TextureSizeMeta, TypeInfo, UboReflection,
    UniformMemberBlock, UniqueSemanticMap, UniqueSemantics, ValidateTypeSemantics, VariableMeta,
    MAX_BINDINGS_COUNT, MAX_PUSH_BUFFER_SIZE,
};
use crate::reflect::{align_uniform_size, ReflectShader, ShaderReflection};

#[derive(Debug)]
pub struct NagaReflect {
    pub(crate) vertex: Module,
    pub(crate) fragment: Module,
}

impl ValidateTypeSemantics<&TypeInner> for UniqueSemantics {
    fn validate_type(&self, ty: &&TypeInner) -> Option<TypeInfo> {
        let (TypeInner::Vector { .. } | TypeInner::Scalar { .. } | TypeInner::Matrix { .. }) = *ty
        else {
            return None;
        };

        match self {
            UniqueSemantics::MVP => {
                if matches!(ty, TypeInner::Matrix { columns, rows, width } if *columns == VectorSize::Quad
                    && *rows == VectorSize::Quad && *width == 4)
                {
                    return Some(TypeInfo {
                        size: 4,
                        columns: 4,
                    });
                }
            }
            UniqueSemantics::FrameCount => {
                // Uint32 == width 4
                if matches!(ty, TypeInner::Scalar { kind, width } if *kind == ScalarKind::Uint && *width == 4)
                {
                    return Some(TypeInfo {
                        size: 1,
                        columns: 1,
                    });
                }
            }
            UniqueSemantics::FrameDirection => {
                // Uint32 == width 4
                if matches!(ty, TypeInner::Scalar { kind, width } if *kind == ScalarKind::Sint && *width == 4)
                {
                    return Some(TypeInfo {
                        size: 1,
                        columns: 1,
                    });
                }
            }
            UniqueSemantics::FloatParameter => {
                // Float32 == width 4
                if matches!(ty, TypeInner::Scalar { kind, width } if *kind == ScalarKind::Float && *width == 4)
                {
                    return Some(TypeInfo {
                        size: 1,
                        columns: 1,
                    });
                }
            }
            _ => {
                if matches!(ty, TypeInner::Vector { kind, width, size } if *kind == ScalarKind::Float && *width == 4 && *size == VectorSize::Quad)
                {
                    return Some(TypeInfo {
                        size: 4,
                        columns: 1,
                    });
                }
            }
        };

        return None;
    }
}

impl ValidateTypeSemantics<&TypeInner> for TextureSemantics {
    fn validate_type(&self, ty: &&TypeInner) -> Option<TypeInfo> {
        let TypeInner::Vector { size, kind, width } = ty else {
            return None;
        };

        if *kind == ScalarKind::Float && *width == 4 && *size == VectorSize::Quad {
            return Some(TypeInfo {
                size: 4,
                columns: 1,
            });
        }

        None
    }
}

impl NagaReflect {
    fn reflect_ubos(
        &mut self,
        vertex_ubo: Option<Handle<GlobalVariable>>,
        fragment_ubo: Option<Handle<GlobalVariable>>,
    ) -> Result<Option<UboReflection>, ShaderReflectError> {
        if let Some(vertex_ubo) = vertex_ubo {
            let ubo = &mut self.vertex.global_variables[vertex_ubo];
            ubo.binding = Some(ResourceBinding {
                group: 0,
                binding: 0,
            })
        }

        if let Some(fragment_ubo) = fragment_ubo {
            let ubo = &mut self.fragment.global_variables[fragment_ubo];
            ubo.binding = Some(ResourceBinding {
                group: 0,
                binding: 0,
            })
        }

        // todo: merge this with the spirv-cross code
        match (vertex_ubo, fragment_ubo) {
            (None, None) => Ok(None),
            (Some(vertex_ubo), Some(fragment_ubo)) => {
                let vertex_ubo = Self::get_ubo_data(
                    &self.vertex,
                    &self.vertex.global_variables[vertex_ubo],
                    SemanticErrorBlame::Vertex,
                )?;
                let fragment_ubo = Self::get_ubo_data(
                    &self.fragment,
                    &self.fragment.global_variables[fragment_ubo],
                    SemanticErrorBlame::Fragment,
                )?;
                if vertex_ubo.binding != fragment_ubo.binding {
                    return Err(ShaderReflectError::MismatchedUniformBuffer {
                        vertex: vertex_ubo.binding,
                        fragment: fragment_ubo.binding,
                    });
                }

                let size = std::cmp::max(vertex_ubo.size, fragment_ubo.size);
                Ok(Some(UboReflection {
                    binding: vertex_ubo.binding,
                    size: align_uniform_size(size),
                    stage_mask: BindingStage::VERTEX | BindingStage::FRAGMENT,
                }))
            }
            (Some(vertex_ubo), None) => {
                let vertex_ubo = Self::get_ubo_data(
                    &self.vertex,
                    &self.vertex.global_variables[vertex_ubo],
                    SemanticErrorBlame::Vertex,
                )?;
                Ok(Some(UboReflection {
                    binding: vertex_ubo.binding,
                    size: align_uniform_size(vertex_ubo.size),
                    stage_mask: BindingStage::VERTEX,
                }))
            }
            (None, Some(fragment_ubo)) => {
                let fragment_ubo = Self::get_ubo_data(
                    &self.fragment,
                    &self.fragment.global_variables[fragment_ubo],
                    SemanticErrorBlame::Fragment,
                )?;
                Ok(Some(UboReflection {
                    binding: fragment_ubo.binding,
                    size: align_uniform_size(fragment_ubo.size),
                    stage_mask: BindingStage::FRAGMENT,
                }))
            }
        }
    }

    fn get_ubo_data(
        module: &Module,
        ubo: &GlobalVariable,
        blame: SemanticErrorBlame,
    ) -> Result<UboData, ShaderReflectError> {
        let Some(binding) = &ubo.binding else {
            return Err(blame.error(SemanticsErrorKind::MissingBinding));
        };

        if binding.binding >= MAX_BINDINGS_COUNT {
            return Err(blame.error(SemanticsErrorKind::InvalidBinding(binding.binding)));
        }

        if binding.group != 0 {
            return Err(blame.error(SemanticsErrorKind::InvalidDescriptorSet(binding.group)));
        }

        let ty = &module.types[ubo.ty];
        let size = ty.inner.size(module.to_ctx());
        Ok(UboData {
            // descriptor_set,
            // id: ubo.id,
            binding: binding.binding,
            size,
        })
    }
    fn get_push_size(
        module: &Module,
        push: &GlobalVariable,
        blame: SemanticErrorBlame,
    ) -> Result<u32, ShaderReflectError> {
        let ty = &module.types[push.ty];
        let size = ty.inner.size(module.to_ctx());
        if size > MAX_PUSH_BUFFER_SIZE {
            return Err(blame.error(SemanticsErrorKind::InvalidPushBufferSize(size)));
        }
        Ok(size)
    }

    fn reflect_push_constant_buffer(
        &mut self,
        vertex_pcb: Option<Handle<GlobalVariable>>,
        fragment_pcb: Option<Handle<GlobalVariable>>,
    ) -> Result<Option<PushReflection>, ShaderReflectError> {
        // Reassign to UBO later if we want during compilation.
        if let Some(vertex_pcb) = vertex_pcb {
            let ubo = &mut self.vertex.global_variables[vertex_pcb];
            ubo.binding = Some(ResourceBinding {
                group: 0,
                binding: 1,
            });
        }

        if let Some(fragment_pcb) = fragment_pcb {
            let ubo = &mut self.fragment.global_variables[fragment_pcb];
            ubo.binding = Some(ResourceBinding {
                group: 0,
                binding: 1,
            });
        };

        match (vertex_pcb, fragment_pcb) {
            (None, None) => Ok(None),
            (Some(vertex_push), Some(fragment_push)) => {
                let vertex_size = Self::get_push_size(
                    &self.vertex,
                    &self.vertex.global_variables[vertex_push],
                    SemanticErrorBlame::Vertex,
                )?;
                let fragment_size = Self::get_push_size(
                    &self.fragment,
                    &self.fragment.global_variables[fragment_push],
                    SemanticErrorBlame::Fragment,
                )?;

                let size = std::cmp::max(vertex_size, fragment_size);

                Ok(Some(PushReflection {
                    size: align_uniform_size(size),
                    stage_mask: BindingStage::VERTEX | BindingStage::FRAGMENT,
                }))
            }
            (Some(vertex_push), None) => {
                let vertex_size = Self::get_push_size(
                    &self.vertex,
                    &self.vertex.global_variables[vertex_push],
                    SemanticErrorBlame::Vertex,
                )?;
                Ok(Some(PushReflection {
                    size: align_uniform_size(vertex_size),
                    stage_mask: BindingStage::VERTEX,
                }))
            }
            (None, Some(fragment_push)) => {
                let fragment_size = Self::get_push_size(
                    &self.fragment,
                    &self.fragment.global_variables[fragment_push],
                    SemanticErrorBlame::Fragment,
                )?;
                Ok(Some(PushReflection {
                    size: align_uniform_size(fragment_size),
                    stage_mask: BindingStage::FRAGMENT,
                }))
            }
        }
    }

    fn validate(&self) -> Result<(), ShaderReflectError> {
        // Verify types
        if self.vertex.global_variables.iter().any(|(_, gv)| {
            let ty = &self.vertex.types[gv.ty];
            match ty.inner {
                TypeInner::Scalar { .. }
                | TypeInner::Vector { .. }
                | TypeInner::Matrix { .. }
                | TypeInner::Struct { .. } => false,
                _ => true,
            }
        }) {
            return Err(ShaderReflectError::VertexSemanticError(
                SemanticsErrorKind::InvalidResourceType,
            ));
        }

        if self.fragment.global_variables.iter().any(|(_, gv)| {
            let ty = &self.fragment.types[gv.ty];
            match ty.inner {
                TypeInner::Scalar { .. }
                | TypeInner::Vector { .. }
                | TypeInner::Matrix { .. }
                | TypeInner::Struct { .. }
                | TypeInner::Image { .. }
                | TypeInner::Sampler { .. } => false,
                TypeInner::BindingArray { base, .. } => {
                    let ty = &self.fragment.types[base];
                    match ty.inner {
                        TypeInner::Image { class, .. }
                            if !matches!(class, ImageClass::Storage { .. }) =>
                        {
                            false
                        }
                        TypeInner::Sampler { .. } => false,
                        _ => true,
                    }
                }
                _ => true,
            }
        }) {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidResourceType,
            ));
        }

        // Verify Vertex inputs
        'vertex: {
            if self.vertex.entry_points.len() != 1 {
                return Err(ShaderReflectError::VertexSemanticError(
                    SemanticsErrorKind::InvalidEntryPointCount(self.vertex.entry_points.len()),
                ));
            }

            let vertex_entry_point = &self.vertex.entry_points[0];
            let vert_inputs = vertex_entry_point.function.arguments.len();
            if vert_inputs != 2 {
                return Err(ShaderReflectError::VertexSemanticError(
                    SemanticsErrorKind::InvalidInputCount(vert_inputs),
                ));
            }
            for input in &vertex_entry_point.function.arguments {
                let &Some(Binding::Location { location, .. }) = &input.binding else {
                    return Err(ShaderReflectError::VertexSemanticError(
                        SemanticsErrorKind::MissingBinding,
                    ));
                };

                if location == 0 {
                    let pos_type = &self.vertex.types[input.ty];
                    if !matches!(pos_type.inner, TypeInner::Vector { size, ..} if size == VectorSize::Quad)
                    {
                        return Err(ShaderReflectError::VertexSemanticError(
                            SemanticsErrorKind::InvalidLocation(location),
                        ));
                    }
                    break 'vertex;
                }

                if location == 1 {
                    let coord_type = &self.vertex.types[input.ty];
                    if !matches!(coord_type.inner, TypeInner::Vector { size, ..} if size == VectorSize::Bi)
                    {
                        return Err(ShaderReflectError::VertexSemanticError(
                            SemanticsErrorKind::InvalidLocation(location),
                        ));
                    }
                    break 'vertex;
                }

                return Err(ShaderReflectError::VertexSemanticError(
                    SemanticsErrorKind::InvalidLocation(location),
                ));
            }

            let uniform_buffer_count = self
                .vertex
                .global_variables
                .iter()
                .filter(|(_, gv)| gv.space == AddressSpace::Uniform)
                .count();

            if uniform_buffer_count > 1 {
                return Err(ShaderReflectError::VertexSemanticError(
                    SemanticsErrorKind::InvalidUniformBufferCount(uniform_buffer_count),
                ));
            }

            let push_buffer_count = self
                .vertex
                .global_variables
                .iter()
                .filter(|(_, gv)| gv.space == AddressSpace::PushConstant)
                .count();

            if push_buffer_count > 1 {
                return Err(ShaderReflectError::VertexSemanticError(
                    SemanticsErrorKind::InvalidPushBufferCount(push_buffer_count),
                ));
            }
        }

        {
            if self.fragment.entry_points.len() != 1 {
                return Err(ShaderReflectError::FragmentSemanticError(
                    SemanticsErrorKind::InvalidEntryPointCount(self.vertex.entry_points.len()),
                ));
            }

            let frag_entry_point = &self.fragment.entry_points[0];
            let Some(frag_output) = &frag_entry_point.function.result else {
                return Err(ShaderReflectError::FragmentSemanticError(
                    SemanticsErrorKind::InvalidOutputCount(0),
                ));
            };

            let &Some(Binding::Location { location, .. }) = &frag_output.binding else {
                return Err(ShaderReflectError::VertexSemanticError(
                    SemanticsErrorKind::MissingBinding,
                ));
            };

            if location != 0 {
                return Err(ShaderReflectError::FragmentSemanticError(
                    SemanticsErrorKind::InvalidLocation(location),
                ));
            }

            let uniform_buffer_count = self
                .fragment
                .global_variables
                .iter()
                .filter(|(_, gv)| gv.space == AddressSpace::Uniform)
                .count();

            if uniform_buffer_count > 1 {
                return Err(ShaderReflectError::FragmentSemanticError(
                    SemanticsErrorKind::InvalidUniformBufferCount(uniform_buffer_count),
                ));
            }

            let push_buffer_count = self
                .fragment
                .global_variables
                .iter()
                .filter(|(_, gv)| gv.space == AddressSpace::PushConstant)
                .count();

            if push_buffer_count > 1 {
                return Err(ShaderReflectError::FragmentSemanticError(
                    SemanticsErrorKind::InvalidPushBufferCount(push_buffer_count),
                ));
            }
        }

        Ok(())
    }

    fn reflect_buffer_struct_members(
        module: &Module,
        resource: Handle<GlobalVariable>,
        pass_number: usize,
        semantics: &ShaderSemantics,
        meta: &mut BindingMeta,
        offset_type: UniformMemberBlock,
        blame: SemanticErrorBlame,
    ) -> Result<(), ShaderReflectError> {
        let resource = &module.global_variables[resource];
        let TypeInner::Struct { members, .. } = &module.types[resource.ty].inner else {
            return Err(blame.error(SemanticsErrorKind::InvalidResourceType));
        };

        for member in members {
            let Some(name) = member.name.clone() else {
                return Err(blame.error(SemanticsErrorKind::InvalidRange(member.offset)));
            };
            let member_type = &module.types[member.ty].inner;

            if let Some(parameter) = semantics.uniform_semantics.get_unique_semantic(&name) {
                let Some(typeinfo) = parameter.semantics.validate_type(&member_type) else {
                    return Err(blame.error(SemanticsErrorKind::InvalidTypeForSemantic(name)));
                };

                match &parameter.semantics {
                    UniqueSemantics::FloatParameter => {
                        let offset = member.offset;
                        if let Some(meta) = meta.parameter_meta.get_mut(&name) {
                            if let Some(expected) = meta.offset.offset(offset_type)
                                && expected != offset as usize
                            {
                                return Err(ShaderReflectError::MismatchedOffset {
                                    semantic: name,
                                    expected,
                                    received: offset as usize,
                                    ty: offset_type,
                                    pass: pass_number,
                                });
                            }
                            if meta.size != typeinfo.size {
                                return Err(ShaderReflectError::MismatchedSize {
                                    semantic: name,
                                    vertex: meta.size,
                                    fragment: typeinfo.size,
                                    pass: pass_number,
                                });
                            }

                            *meta.offset.offset_mut(offset_type) = Some(offset as usize);
                        } else {
                            meta.parameter_meta.insert(
                                name.clone(),
                                VariableMeta {
                                    id: name,
                                    offset: MemberOffset::new(offset as usize, offset_type),
                                    size: typeinfo.size,
                                },
                            );
                        }
                    }
                    semantics => {
                        let offset = member.offset;
                        if let Some(meta) = meta.unique_meta.get_mut(semantics) {
                            if let Some(expected) = meta.offset.offset(offset_type)
                                && expected != offset as usize
                            {
                                return Err(ShaderReflectError::MismatchedOffset {
                                    semantic: name,
                                    expected,
                                    received: offset as usize,
                                    ty: offset_type,
                                    pass: pass_number,
                                });
                            }
                            if meta.size != typeinfo.size * typeinfo.columns {
                                return Err(ShaderReflectError::MismatchedSize {
                                    semantic: name,
                                    vertex: meta.size,
                                    fragment: typeinfo.size,
                                    pass: pass_number,
                                });
                            }

                            *meta.offset.offset_mut(offset_type) = Some(offset as usize);
                        } else {
                            meta.unique_meta.insert(
                                *semantics,
                                VariableMeta {
                                    id: name,
                                    offset: MemberOffset::new(offset as usize, offset_type),
                                    size: typeinfo.size * typeinfo.columns,
                                },
                            );
                        }
                    }
                }
            } else if let Some(texture) = semantics.uniform_semantics.get_texture_semantic(&name) {
                let Some(_typeinfo) = texture.semantics.validate_type(&member_type) else {
                    return Err(blame.error(SemanticsErrorKind::InvalidTypeForSemantic(name)));
                };

                if let TextureSemantics::PassOutput = texture.semantics {
                    if texture.index >= pass_number {
                        return Err(ShaderReflectError::NonCausalFilterChain {
                            pass: pass_number,
                            target: texture.index,
                        });
                    }
                }

                let offset = member.offset;
                if let Some(meta) = meta.texture_size_meta.get_mut(&texture) {
                    if let Some(expected) = meta.offset.offset(offset_type)
                        && expected != offset as usize
                    {
                        return Err(ShaderReflectError::MismatchedOffset {
                            semantic: name,
                            expected,
                            received: offset as usize,
                            ty: offset_type,
                            pass: pass_number,
                        });
                    }

                    meta.stage_mask.insert(match blame {
                        SemanticErrorBlame::Vertex => BindingStage::VERTEX,
                        SemanticErrorBlame::Fragment => BindingStage::FRAGMENT,
                    });

                    *meta.offset.offset_mut(offset_type) = Some(offset as usize);
                } else {
                    meta.texture_size_meta.insert(
                        texture,
                        TextureSizeMeta {
                            offset: MemberOffset::new(offset as usize, offset_type),
                            stage_mask: match blame {
                                SemanticErrorBlame::Vertex => BindingStage::VERTEX,
                                SemanticErrorBlame::Fragment => BindingStage::FRAGMENT,
                            },
                            id: name,
                        },
                    );
                }
            } else {
                return Err(blame.error(SemanticsErrorKind::UnknownSemantics(name)));
            }
        }
        Ok(())
    }

    fn reflect_texture<'a>(
        &'a self,
        texture: &'a GlobalVariable,
    ) -> Result<TextureData<'a>, ShaderReflectError> {
        let Some(binding) = &texture.binding else {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::MissingBinding,
            ));
        };

        let Some(name) = texture.name.as_ref() else {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidBinding(binding.binding),
            ));
        };

        if binding.group != 0 {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidDescriptorSet(binding.group),
            ));
        }
        if binding.binding >= MAX_BINDINGS_COUNT {
            return Err(ShaderReflectError::FragmentSemanticError(
                SemanticsErrorKind::InvalidBinding(binding.binding),
            ));
        }

        Ok(TextureData {
            // id: texture.id,
            // descriptor_set,
            name: &name,
            binding: binding.binding,
        })
    }

    // todo: share this with cross
    fn reflect_texture_metas(
        &self,
        texture: TextureData,
        pass_number: usize,
        semantics: &ShaderSemantics,
        meta: &mut BindingMeta,
    ) -> Result<(), ShaderReflectError> {
        let Some(semantic) = semantics
            .texture_semantics
            .get_texture_semantic(texture.name)
        else {
            return Err(
                SemanticErrorBlame::Fragment.error(SemanticsErrorKind::UnknownSemantics(
                    texture.name.to_string(),
                )),
            );
        };

        if semantic.semantics == TextureSemantics::PassOutput && semantic.index >= pass_number {
            return Err(ShaderReflectError::NonCausalFilterChain {
                pass: pass_number,
                target: semantic.index,
            });
        }

        meta.texture_meta.insert(
            semantic,
            TextureBinding {
                binding: texture.binding,
            },
        );
        Ok(())
    }
}

impl ReflectShader for NagaReflect {
    fn reflect(
        &mut self,
        pass_number: usize,
        semantics: &ShaderSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError> {
        self.validate()?;

        // Validate verifies that there's only one uniform block.
        let vertex_ubo = self
            .vertex
            .global_variables
            .iter()
            .find_map(|(handle, gv)| {
                if gv.space == AddressSpace::Uniform {
                    Some(handle)
                } else {
                    None
                }
            });

        let fragment_ubo = self
            .fragment
            .global_variables
            .iter()
            .find_map(|(handle, gv)| {
                if gv.space == AddressSpace::Uniform {
                    Some(handle)
                } else {
                    None
                }
            });

        let ubo = self.reflect_ubos(vertex_ubo, fragment_ubo)?;

        let vertex_push = self
            .vertex
            .global_variables
            .iter()
            .find_map(|(handle, gv)| {
                if gv.space == AddressSpace::PushConstant {
                    Some(handle)
                } else {
                    None
                }
            });

        let fragment_push = self
            .fragment
            .global_variables
            .iter()
            .find_map(|(handle, gv)| {
                if gv.space == AddressSpace::PushConstant {
                    Some(handle)
                } else {
                    None
                }
            });

        let push_constant = self.reflect_push_constant_buffer(vertex_push, fragment_push)?;

        let mut meta = BindingMeta::default();

        if let Some(ubo) = vertex_ubo {
            Self::reflect_buffer_struct_members(
                &self.vertex,
                ubo,
                pass_number,
                semantics,
                &mut meta,
                UniformMemberBlock::Ubo,
                SemanticErrorBlame::Vertex,
            )?;
        }

        if let Some(ubo) = fragment_ubo {
            Self::reflect_buffer_struct_members(
                &self.fragment,
                ubo,
                pass_number,
                semantics,
                &mut meta,
                UniformMemberBlock::Ubo,
                SemanticErrorBlame::Fragment,
            )?;
        }

        if let Some(push) = vertex_push {
            Self::reflect_buffer_struct_members(
                &self.vertex,
                push,
                pass_number,
                semantics,
                &mut meta,
                UniformMemberBlock::PushConstant,
                SemanticErrorBlame::Vertex,
            )?;
        }

        if let Some(push) = fragment_push {
            Self::reflect_buffer_struct_members(
                &self.fragment,
                push,
                pass_number,
                semantics,
                &mut meta,
                UniformMemberBlock::PushConstant,
                SemanticErrorBlame::Fragment,
            )?;
        }

        let mut ubo_bindings = 0u16;
        if vertex_ubo.is_some() || fragment_ubo.is_some() {
            ubo_bindings = 1 << ubo.as_ref().expect("UBOs should be present").binding;
        }

        let textures = self.fragment.global_variables.iter().filter(|(_, gv)| {
            let ty = &self.fragment.types[gv.ty];
            matches!(ty.inner, TypeInner::Image { .. })
        });

        for (_, texture) in textures {
            let texture_data = self.reflect_texture(texture)?;
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
