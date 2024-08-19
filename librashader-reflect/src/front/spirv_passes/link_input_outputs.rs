use rspirv::dr::{Builder, Module, Operand};
use rustc_hash::{FxHashMap};
use spirv::{Decoration, Op, StorageClass};

/// Do DCE on inputs and link
pub struct LinkInputs<'a> {
    pub frag_builder: &'a mut Builder,
    pub vert_builder: &'a mut Builder,

    pub outputs: Vec<(u32, spirv::Word)>,
    // pub inputs: Vec<(u32, spirv::Word)>,
    pub inputs_to_remove: FxHashMap<spirv::Word, u32>,
}

impl<'a> LinkInputs<'a> {

    /// Get the value of the location of the inout in the module
    fn find_location(module: &Module, id: spirv::Word) -> Option<u32> {
        module.annotations.iter().find_map(|op| {
            if op.class.opcode != Op::Decorate {
                return None;
            }

            let Some(Operand::Decoration(Decoration::Location)) = op.operands.get(1) else {
                return None;
            };

            let Some(&Operand::IdRef(target)) = op.operands.get(0) else {
                return None;
            };

            if target != id {
                return None;
            }

            let Some(&Operand::LiteralBit32(binding)) = op.operands.get(2) else {
                return None;
            };
            return Some(binding);
        })
    }

    /// Get a mutable reference to the inout in the module
    fn find_location_operand(module: &mut Module, id: spirv::Word) -> Option<&mut u32> {
        module.annotations.iter_mut().find_map(|op| {
            if op.class.opcode != Op::Decorate {
                return None;
            }

            let Some(Operand::Decoration(Decoration::Location)) = op.operands.get(1) else {
                return None;
            };

            let Some(&Operand::IdRef(target)) = op.operands.get(0) else {
                return None;
            };

            if target != id {
                return None;
            }

            let Some(Operand::LiteralBit32(binding)) = op.operands.get_mut(2) else {
                return None;
            };
            return Some(binding);
        })
    }

    pub fn new(vert: &'a mut Builder, frag: &'a mut Builder, keep_if_bound: bool) -> Self {
        let mut outputs = FxHashMap::default();
        let mut inputs_to_remove = FxHashMap::default();
        // let mut inputs = FxHashMap::default();

        for global in frag.module_ref().types_global_values.iter() {
            if global.class.opcode == spirv::Op::Variable
                && global.operands[0] == Operand::StorageClass(StorageClass::Input)
            {
                if let Some(id) = global.result_id {
                    let Some(location) = Self::find_location(frag.module_ref(), id) else {
                        continue;
                    };

                    inputs_to_remove.insert(id, location);
                    // inputs.insert(location, id);
                }
            }
        }

        for global in vert.module_ref().types_global_values.iter() {
            if global.class.opcode == spirv::Op::Variable
                && global.operands[0] == Operand::StorageClass(StorageClass::Output)
            {
                if let Some(id) = global.result_id {
                    let Some(location) = Self::find_location(vert.module_ref(), id) else {
                        continue;
                    };

                    // Add to list of outputs
                    outputs.insert(location, id);

                    // Keep the input, if it is bound to both stages.
                    // Otherwise, do DCE analysis on the input, and remove it
                    // regardless if bound.
                    if keep_if_bound {
                        if let Some(&frag_ref) = inputs_to_remove.get(&location) {
                            // if something is bound to the same location in the vertex shader,
                            // we're good.
                            inputs_to_remove.remove(&frag_ref);
                        }
                    }
                }
            }
        }

        let mut outputs: Vec<(u32, spirv::Word)> = outputs.into_iter().collect();
        // let mut inputs: Vec<(u32, spirv::Word)> = inputs.into_iter().collect();

        outputs.sort_by(|&(a, _), &(b, _)| a.cmp(&b));

        Self {
            frag_builder: frag,
            vert_builder: vert,
            outputs,
            // inputs,
            inputs_to_remove,
        }
    }


    pub fn do_pass(&mut self) {
        self.trim_inputs();
        self.reorder_inputs();
    }

   fn trim_inputs(&mut self) {
        let functions = &self.frag_builder.module_ref().functions;

        // literally if it has any reference at all we can keep it
        for function in functions {
            for param in &function.parameters {
                for op in &param.operands {
                    if let Some(word) = op.id_ref_any() {
                        if self.inputs_to_remove.contains_key(&word) {
                            self.inputs_to_remove.remove(&word);
                        }
                    }
                }
            }

            for block in &function.blocks {
                for inst in &block.instructions {
                    for op in &inst.operands {
                        if let Some(word) = op.id_ref_any() {
                            if self.inputs_to_remove.contains_key(&word) {
                                self.inputs_to_remove.remove(&word);
                            }
                        }
                    }
                }
            }
        }

        // ok well guess we dont

        self.frag_builder.module_mut().debug_names.retain(|instr| {
            for op in &instr.operands {
                if let Some(word) = op.id_ref_any() {
                    if self.inputs_to_remove.contains_key(&word) {
                        return false;
                    }
                }
            }
            return true;
        });

        self.frag_builder.module_mut().annotations.retain(|instr| {
            for op in &instr.operands {
                if let Some(word) = op.id_ref_any() {
                    if self.inputs_to_remove.contains_key(&word) {
                        return false;
                    }
                }
            }
            return true;
        });

        for entry_point in self.frag_builder.module_mut().entry_points.iter_mut() {
            entry_point.operands.retain(|op| {
                if let Some(word) = op.id_ref_any() {
                    if self.inputs_to_remove.contains_key(&word) {
                        return false;
                    }
                }
                return true;
            })
        }

        self.frag_builder
            .module_mut()
            .types_global_values
            .retain(|instr| {
                let Some(id) = instr.result_id else {
                    return true;
                };

                !self.inputs_to_remove.contains_key(&id)
            });
    }

    fn reorder_inputs(&mut self) {
        // Preconditions:
        //   - trim_inputs is called, so all dead inputs are gone from the frag builder


        // We want to have all the dead inputs get ordered last, but otherwise ensure
        // that all locations are ordered.

        let mut dead_inputs = self.inputs_to_remove.values().collect::<Vec<_>>();
        dead_inputs.sort();

        let Some(&&first) = dead_inputs.first() else {
            // If there are no dead inputs then things are contiguous
            return;
        };

        // Mapping of old bindings -> new bindings
        let mut remapping = FxHashMap::default();

        // Start at the first dead input
        let mut alloc = first;

        for (binding, _) in &self.outputs {
            if *binding < alloc {
                continue
            }

            if !dead_inputs.contains(&binding) {
                remapping.insert(*binding, alloc);
                alloc += 1;
            }
        }

        // Now assign dead inputs the end
        for binding in &dead_inputs {
            remapping.insert(**binding, alloc);
            alloc += 1;
        }

        // eprintln!("dead: {:#?}", dead_inputs);

        // eprintln!("remapping: {:#?}", remapping);

        let frag_clone = self.frag_builder.module_ref().clone();
        let frag_mut = self.frag_builder.module_mut();

        for global in frag_clone.types_global_values {
            if global.class.opcode == spirv::Op::Variable
                && global.operands[0] == Operand::StorageClass(StorageClass::Input)
            {
                if let Some(id) = global.result_id {
                    let Some(location) = Self::find_location_operand(frag_mut, id) else {
                        continue;
                    };

                    let Some(&remapping) = remapping.get(&location) else {
                        continue
                    };

                    // eprintln!("frag: remapped {} to {}", *location, remapping);

                    *location = remapping;
                }
            }
        }

        let vert_clone = self.vert_builder.module_ref().clone();
        let vert_mut = self.vert_builder.module_mut();

        for global in vert_clone.types_global_values {
            if global.class.opcode == spirv::Op::Variable
                && global.operands[0] == Operand::StorageClass(StorageClass::Output)
            {
                if let Some(id) = global.result_id {
                    let Some(location) = Self::find_location_operand(vert_mut, id) else {
                        continue;
                    };

                    let Some(&remapping) = remapping.get(&location) else {
                        continue
                    };

                    // eprintln!("vert: remapped {} to {}", *location, remapping);
                    *location = remapping;
                }
            }
        }
    }
}
