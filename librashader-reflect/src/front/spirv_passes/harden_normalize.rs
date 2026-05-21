//! Workaround for an FXC bug where SM3 rejects NaN/inf literals that the
//! compiler itself produces via aggressive constant folding.
//!
//! Walking the SPIR-V, this pass wraps every `OpExtInst Normalize %vec`
//! with an `OpFAdd %vec %eps` to work around the FXC bug.
//!
//! For non-zero inputs the perturbation (≈1e-7 per component) is
//! lost in normalization; for zero inputs the result is now a finite
//! near-unit vector instead of NaN.
//!
//! This pass should only be run when generating SM3 HLSL.

use rspirv::dr::{Builder, Instruction, Operand};
use rustc_hash::{FxHashMap, FxHashSet};
use spirv::{Op, Word};

/// Per-component epsilon added to vectors before they are passed to `Normalize`.
/// Small enough to be lost in re-normalization for non-zero inputs, large enough
/// to keep `normalize((0,..,0))` from collapsing to NaN under FXC's folding.
const NORMALIZE_EPSILON: f32 = 1e-7;

pub struct HardenNormalize<'a> {
    pub builder: &'a mut Builder,
}

impl<'a> HardenNormalize<'a> {
    pub fn new(builder: &'a mut Builder) -> Self {
        Self { builder }
    }

    pub fn do_pass(&mut self) {
        let Some(ext_set) = self.find_glsl_std_450() else {
            return;
        };

        let result_types = self.collect_normalize_result_types(ext_set);
        if result_types.is_empty() {
            return;
        }

        let epsilon_constants = self.create_epsilon_constants(&result_types);
        self.rewrite_normalize_calls(ext_set, &epsilon_constants);
    }

    fn find_glsl_std_450(&self) -> Option<Word> {
        for instr in &self.builder.module_ref().ext_inst_imports {
            if instr.class.opcode != Op::ExtInstImport {
                continue;
            }
            if let Some(Operand::LiteralString(name)) = instr.operands.first() {
                if name == "GLSL.std.450" {
                    return instr.result_id;
                }
            }
        }
        None
    }

    /// Collect the result-type id of every `OpExtInst Normalize` in the module.
    /// For `Normalize` the result type equals the input vector type.
    fn collect_normalize_result_types(&self, ext_set: Word) -> FxHashSet<Word> {
        let mut types = FxHashSet::default();
        for function in &self.builder.module_ref().functions {
            for block in &function.blocks {
                for instr in &block.instructions {
                    if !is_normalize(instr, ext_set) {
                        continue;
                    }
                    if let Some(result_type) = instr.result_type {
                        types.insert(result_type);
                    }
                }
            }
        }
        types
    }

    /// Build (or look up) an epsilon constant for each vector/scalar type used
    /// as a `Normalize` operand. Returns a map from type id → constant id.
    fn create_epsilon_constants(&mut self, types: &FxHashSet<Word>) -> FxHashMap<Word, Word> {
        let mut result = FxHashMap::default();
        // Cache scalar epsilons by float-type id so we don't rebuild them.
        let mut scalar_eps_by_float_type: FxHashMap<Word, Word> = FxHashMap::default();

        for &type_id in types {
            let Some((float_type, component_count)) = self.float_type_and_count(type_id) else {
                continue;
            };

            let eps_scalar = *scalar_eps_by_float_type
                .entry(float_type)
                .or_insert_with(|| {
                    self.builder
                        .constant_bit32(float_type, NORMALIZE_EPSILON.to_bits())
                });

            let composite = if component_count == 1 {
                eps_scalar
            } else {
                let constituents = vec![eps_scalar; component_count as usize];
                self.builder.constant_composite(type_id, constituents)
            };

            result.insert(type_id, composite);
        }

        result
    }

    /// Resolve `type_id` to its underlying `(float_type_id, component_count)`.
    /// Returns `None` if the type isn't a float scalar or float vector.
    fn float_type_and_count(&self, type_id: Word) -> Option<(Word, u32)> {
        let type_instr = self
            .builder
            .module_ref()
            .types_global_values
            .iter()
            .find(|i| i.result_id == Some(type_id))?;

        match type_instr.class.opcode {
            Op::TypeFloat => Some((type_id, 1)),
            Op::TypeVector => {
                let component_type = type_instr.operands.first()?.id_ref_any()?;
                let count = match type_instr.operands.get(1)? {
                    Operand::LiteralBit32(n) => *n,
                    _ => return None,
                };

                // Verify the component type is actually a float — Normalize on
                // integer vectors is invalid SPIR-V, but be defensive.
                let comp_instr = self
                    .builder
                    .module_ref()
                    .types_global_values
                    .iter()
                    .find(|i| i.result_id == Some(component_type))?;
                if comp_instr.class.opcode != Op::TypeFloat {
                    return None;
                }
                Some((component_type, count))
            }
            _ => None,
        }
    }

    /// For every `OpExtInst Normalize %input`, insert
    /// `%input' = OpFAdd %input %eps` just before it and rewrite the operand.
    fn rewrite_normalize_calls(
        &mut self,
        ext_set: Word,
        epsilon_constants: &FxHashMap<Word, Word>,
    ) {
        let mut functions = std::mem::take(&mut self.builder.module_mut().functions);

        for function in functions.iter_mut() {
            for block in function.blocks.iter_mut() {
                let mut new_instructions = Vec::with_capacity(block.instructions.len());
                for instr in block.instructions.drain(..) {
                    if !is_normalize(&instr, ext_set) {
                        new_instructions.push(instr);
                        continue;
                    }

                    let Some(result_type) = instr.result_type else {
                        new_instructions.push(instr);
                        continue;
                    };
                    let Some(&eps_id) = epsilon_constants.get(&result_type) else {
                        new_instructions.push(instr);
                        continue;
                    };
                    let Some(Operand::IdRef(input_id)) = instr.operands.get(2).cloned() else {
                        new_instructions.push(instr);
                        continue;
                    };

                    let fadd_id = self.builder.id();
                    new_instructions.push(Instruction {
                        class: rspirv::grammar::CoreInstructionTable::get(Op::FAdd),
                        result_type: Some(result_type),
                        result_id: Some(fadd_id),
                        operands: vec![Operand::IdRef(input_id), Operand::IdRef(eps_id)],
                    });

                    let mut new_normalize = instr;
                    new_normalize.operands[2] = Operand::IdRef(fadd_id);
                    new_instructions.push(new_normalize);
                }
                block.instructions = new_instructions;
            }
        }

        self.builder.module_mut().functions = functions;
    }
}

fn is_normalize(instr: &Instruction, ext_set: Word) -> bool {
    if instr.class.opcode != Op::ExtInst {
        return false;
    }
    let Some(Operand::IdRef(set)) = instr.operands.first() else {
        return false;
    };
    if *set != ext_set {
        return false;
    }
    let Some(Operand::LiteralExtInstInteger(opc)) = instr.operands.get(1) else {
        return false;
    };
    *opc == spirv::GLOp::Normalize as u32
}
