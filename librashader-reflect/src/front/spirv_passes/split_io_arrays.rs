use rspirv::dr::{Builder, Instruction, Module, Operand};
use spirv::{Decoration, Op, StorageClass};

/// Split aggregate (array) shader I/O variables into N scalar/vector
/// variables at consecutive locations.
///
/// WGSL forbids non-scalar/non-vector types at the user-defined I/O
/// interface between stages, so naga rejects modules that pass arrays
/// across the vertex/fragment boundary. SPIR-V emitted from GLSL `out
/// float arr[N]` declares a single Output (or Input) variable of array
/// type with one Location, occupying `N` consecutive locations. This
/// pass rewrites each such variable into `N` independent location
/// variables and inserts scatter/gather copies in `main()` so the
/// shader body, which still indexes the original array, continues to
/// work unchanged.
///
/// The original array variable is demoted to `Private` storage so that
/// existing `OpAccessChain`/`OpLoad`/`OpStore` instructions referencing
/// it remain valid (their result-type pointer classes are updated to
/// `Private`).
///
/// `inject_scatter`/`inject_gather` then adds loads/stores to read/write from
/// the original array variable into the I/O variables.
pub struct SplitArrayIo<'a> {
    pub builder: &'a mut Builder,
    pub io_class: StorageClass,
}

#[derive(Debug, Clone)]
struct ArrayIoVar {
    var_id: spirv::Word,
    array_type_id: spirv::Word,
    element_type_id: spirv::Word,
    array_length: u32,
    location: u32,
    /// Decorations that should be replicated on each scalar variable
    /// (interpolation, sampling, etc. — everything except Location/Component).
    replicated_decorations: Vec<Instruction>,
}

impl<'a> SplitArrayIo<'a> {
    pub fn new(builder: &'a mut Builder, io_class: StorageClass) -> Self {
        debug_assert!(matches!(
            io_class,
            StorageClass::Input | StorageClass::Output
        ));
        Self { builder, io_class }
    }

    pub fn do_pass(&mut self) {
        let arrays = self.collect_array_io_vars();
        if arrays.is_empty() {
            return;
        }

        for arr in arrays {
            self.split_one(&arr);
        }

        self.put_variables_to_end();
    }

    fn collect_array_io_vars(&self) -> Vec<ArrayIoVar> {
        let module = self.builder.module_ref();
        let mut result = Vec::new();

        for global in module.types_global_values.iter() {
            if global.class.opcode != Op::Variable {
                continue;
            }
            if global.operands.first() != Some(&Operand::StorageClass(self.io_class)) {
                continue;
            }

            let Some(var_id) = global.result_id else {
                continue;
            };
            let Some(ptr_type_id) = global.result_type else {
                continue;
            };

            let Some(ptr_type) = Self::find_global(module, ptr_type_id) else {
                continue;
            };
            if ptr_type.class.opcode != Op::TypePointer {
                continue;
            }
            let Some(&Operand::IdRef(pointee_type_id)) = ptr_type.operands.get(1) else {
                continue;
            };

            let Some(array_type) = Self::find_global(module, pointee_type_id) else {
                continue;
            };
            if array_type.class.opcode != Op::TypeArray {
                continue;
            }
            let Some(&Operand::IdRef(element_type_id)) = array_type.operands.get(0) else {
                continue;
            };
            let Some(&Operand::IdRef(length_id)) = array_type.operands.get(1) else {
                continue;
            };
            let Some(array_length) = Self::resolve_uint_constant(module, length_id) else {
                continue;
            };
            if array_length == 0 {
                continue;
            }

            let mut location = None;
            let mut replicated_decorations = Vec::new();
            for ann in module.annotations.iter() {
                if ann.class.opcode != Op::Decorate {
                    continue;
                }
                let Some(&Operand::IdRef(target)) = ann.operands.first() else {
                    continue;
                };
                if target != var_id {
                    continue;
                }
                let Some(Operand::Decoration(deco)) = ann.operands.get(1) else {
                    continue;
                };
                match deco {
                    Decoration::Location => {
                        if let Some(&Operand::LiteralBit32(loc)) = ann.operands.get(2) {
                            location = Some(loc);
                        }
                    }
                    // These propagate to each scalar variable.
                    Decoration::Flat
                    | Decoration::NoPerspective
                    | Decoration::Centroid
                    | Decoration::Sample
                    | Decoration::Invariant
                    | Decoration::Patch => {
                        replicated_decorations.push(ann.clone());
                    }
                    _ => {}
                }
            }
            let Some(location) = location else {
                continue;
            };

            result.push(ArrayIoVar {
                var_id,
                array_type_id: pointee_type_id,
                element_type_id,
                array_length,
                location,
                replicated_decorations,
            });
        }

        result
    }

    fn split_one(&mut self, arr: &ArrayIoVar) {
        // Pre-create all new IO pointer / variable types up front so subsequent
        // mutations of types_global_values don't invalidate the iteration we'd
        // need to do otherwise.
        let scalar_io_ptr =
            self.builder
                .type_pointer(None, self.io_class, arr.element_type_id);

        let mut scalar_vars = Vec::with_capacity(arr.array_length as usize);
        for i in 0..arr.array_length {
            let var = self
                .builder
                .variable(scalar_io_ptr, None, self.io_class, None);
            self.builder.decorate(
                var,
                Decoration::Location,
                std::iter::once(Operand::LiteralBit32(arr.location + i)),
            );
            for deco in &arr.replicated_decorations {
                let Operand::Decoration(deco_type) = deco.operands[1] else {
                    continue;
                };
                self.builder
                    .decorate(var, deco_type, deco.operands[2..].iter().cloned());
            }
            scalar_vars.push(var);
        }

        // Demote the original array variable to Private and prepare pointer
        // types for the rewritten accesses.
        let private_array_ptr =
            self.builder
                .type_pointer(None, StorageClass::Private, arr.array_type_id);
        let private_element_ptr =
            self.builder
                .type_pointer(None, StorageClass::Private, arr.element_type_id);
        let uint_type = self.builder.type_int(32, 0);
        let index_constants: Vec<spirv::Word> = (0..arr.array_length)
            .map(|i| self.builder.constant_bit32(uint_type, i))
            .collect();

        // Update the OpVariable and all access chains that reference it.
        for global in self.builder.module_mut().types_global_values.iter_mut() {
            if global.class.opcode == Op::Variable && global.result_id == Some(arr.var_id) {
                global.result_type = Some(private_array_ptr);
                if let Some(op) = global.operands.first_mut() {
                    *op = Operand::StorageClass(StorageClass::Private);
                }
                break;
            }
        }

        for function in self.builder.module_mut().functions.iter_mut() {
            for block in function.blocks.iter_mut() {
                for instr in block.instructions.iter_mut() {
                    if !matches!(
                        instr.class.opcode,
                        Op::AccessChain | Op::InBoundsAccessChain | Op::PtrAccessChain
                    ) {
                        continue;
                    }
                    let Some(&Operand::IdRef(base)) = instr.operands.first() else {
                        continue;
                    };
                    if base != arr.var_id {
                        continue;
                    }
                    instr.result_type = Some(private_element_ptr);
                }
            }
        }

        // Strip I/O-only decorations from the demoted variable. They are
        // forbidden on Private variables and would fail validation.
        let var_id = arr.var_id;
        self.builder.module_mut().annotations.retain(|ann| {
            if ann.class.opcode != Op::Decorate {
                return true;
            }
            let Some(&Operand::IdRef(target)) = ann.operands.first() else {
                return true;
            };
            if target != var_id {
                return true;
            }
            let Some(Operand::Decoration(deco)) = ann.operands.get(1) else {
                return true;
            };
            !matches!(
                deco,
                Decoration::Location
                    | Decoration::Component
                    | Decoration::Index
                    | Decoration::NoPerspective
                    | Decoration::Flat
                    | Decoration::Centroid
                    | Decoration::Sample
                    | Decoration::Patch
                    | Decoration::Invariant
                    | Decoration::Stream
                    | Decoration::XfbBuffer
                    | Decoration::XfbStride
            )
        });

        // Replace the array var with the new scalar vars in every entry-point
        // interface list. OpEntryPoint operands: [ExecutionModel, EntryPointId,
        // Name, Interface...]; only IdRef entries beyond Name are interface
        // variables.
        for entry_point in self.builder.module_mut().entry_points.iter_mut() {
            let mut rebuilt: Vec<Operand> = Vec::with_capacity(entry_point.operands.len());
            for (idx, op) in entry_point.operands.iter().enumerate() {
                if idx >= 3 {
                    if let Operand::IdRef(id) = op {
                        if *id == var_id {
                            for &sv in &scalar_vars {
                                rebuilt.push(Operand::IdRef(sv));
                            }
                            continue;
                        }
                    }
                }
                rebuilt.push(op.clone());
            }
            entry_point.operands = rebuilt;
        }

        // Find main entry point(s) referencing this variable and inject
        // scatter (Output) or gather (Input) code.
        let main_ids = self.entry_point_function_ids();
        for main_id in main_ids {
            match self.io_class {
                StorageClass::Output => self.inject_scatter(
                    main_id,
                    arr,
                    &scalar_vars,
                    &index_constants,
                    private_element_ptr,
                ),
                StorageClass::Input => self.inject_gather(
                    main_id,
                    arr,
                    &scalar_vars,
                    &index_constants,
                    private_element_ptr,
                ),
                _ => {}
            }
        }
    }

    fn inject_scatter(
        &mut self,
        main_id: spirv::Word,
        arr: &ArrayIoVar,
        scalar_vars: &[spirv::Word],
        index_constants: &[spirv::Word],
        private_element_ptr: spirv::Word,
    ) {
        // Pre-allocate all IDs so we don't need a `&mut self` mid-iteration.
        let ids: Vec<(spirv::Word, spirv::Word)> = (0..arr.array_length)
            .map(|_| (self.builder.id(), self.builder.id()))
            .collect();

        for function in self.builder.module_mut().functions.iter_mut() {
            if function.def.as_ref().and_then(|d| d.result_id) != Some(main_id) {
                continue;
            }
            for block in function.blocks.iter_mut() {
                let Some(term_idx) = block.instructions.iter().position(|i| {
                    matches!(
                        i.class.opcode,
                        Op::Return | Op::ReturnValue | Op::Kill | Op::Unreachable
                    )
                }) else {
                    continue;
                };
                let mut inserted = Vec::with_capacity(arr.array_length as usize * 3);
                for i in 0..arr.array_length as usize {
                    let (ptr_id, val_id) = ids[i];
                    inserted.push(Instruction::new(
                        Op::AccessChain,
                        Some(private_element_ptr),
                        Some(ptr_id),
                        vec![
                            Operand::IdRef(arr.var_id),
                            Operand::IdRef(index_constants[i]),
                        ],
                    ));
                    inserted.push(Instruction::new(
                        Op::Load,
                        Some(arr.element_type_id),
                        Some(val_id),
                        vec![Operand::IdRef(ptr_id)],
                    ));
                    inserted.push(Instruction::new(
                        Op::Store,
                        None,
                        None,
                        vec![Operand::IdRef(scalar_vars[i]), Operand::IdRef(val_id)],
                    ));
                }
                let tail = block.instructions.split_off(term_idx);
                block.instructions.extend(inserted);
                block.instructions.extend(tail);
                // Only inject into the first return-bearing block we find;
                // subsequent reuse of the same pre-allocated IDs would create
                // duplicate definitions.
                break;
            }
            break;
        }
    }

    fn inject_gather(
        &mut self,
        main_id: spirv::Word,
        arr: &ArrayIoVar,
        scalar_vars: &[spirv::Word],
        index_constants: &[spirv::Word],
        private_element_ptr: spirv::Word,
    ) {
        let ids: Vec<(spirv::Word, spirv::Word)> = (0..arr.array_length)
            .map(|_| (self.builder.id(), self.builder.id()))
            .collect();

        for function in self.builder.module_mut().functions.iter_mut() {
            if function.def.as_ref().and_then(|d| d.result_id) != Some(main_id) {
                continue;
            }
            let Some(block) = function.blocks.first_mut() else {
                continue;
            };
            // Skip the leading OpVariable instructions (which must come first
            // in the entry block per SPIR-V layout rules) before inserting.
            let insert_at = block
                .instructions
                .iter()
                .position(|i| i.class.opcode != Op::Variable)
                .unwrap_or(block.instructions.len());
            let mut inserted = Vec::with_capacity(arr.array_length as usize * 3);
            for i in 0..arr.array_length as usize {
                let (val_id, ptr_id) = ids[i];
                inserted.push(Instruction::new(
                    Op::Load,
                    Some(arr.element_type_id),
                    Some(val_id),
                    vec![Operand::IdRef(scalar_vars[i])],
                ));
                inserted.push(Instruction::new(
                    Op::AccessChain,
                    Some(private_element_ptr),
                    Some(ptr_id),
                    vec![
                        Operand::IdRef(arr.var_id),
                        Operand::IdRef(index_constants[i]),
                    ],
                ));
                inserted.push(Instruction::new(
                    Op::Store,
                    None,
                    None,
                    vec![Operand::IdRef(ptr_id), Operand::IdRef(val_id)],
                ));
            }
            let tail = block.instructions.split_off(insert_at);
            block.instructions.extend(inserted);
            block.instructions.extend(tail);
            break;
        }
    }

    fn entry_point_function_ids(&self) -> Vec<spirv::Word> {
        self.builder
            .module_ref()
            .entry_points
            .iter()
            .filter_map(|ep| match ep.operands.get(1) {
                Some(&Operand::IdRef(id)) => Some(id),
                _ => None,
            })
            .collect()
    }

    fn find_global(module: &Module, id: spirv::Word) -> Option<&Instruction> {
        module
            .types_global_values
            .iter()
            .find(|i| i.result_id == Some(id))
    }

    fn resolve_uint_constant(module: &Module, id: spirv::Word) -> Option<u32> {
        let inst = Self::find_global(module, id)?;
        if inst.class.opcode != Op::Constant {
            return None;
        }
        match inst.operands.first()? {
            &Operand::LiteralBit32(v) => Some(v),
            _ => None,
        }
    }

    /// Move every OpVariable after every type/constant declaration so that
    /// freshly created Private pointer types precede the demoted variable
    /// that now uses them. SPIR-V requires every operand to be declared
    /// before its use within `types_global_values`.
    fn put_variables_to_end(&mut self) {
        let mut vars = Vec::new();
        self.builder
            .module_mut()
            .types_global_values
            .retain(|instr| {
                if instr.class.opcode == Op::Variable {
                    vars.push(instr.clone());
                    return false;
                }
                true
            });
        self.builder.module_mut().types_global_values.extend(vars);
    }
}
