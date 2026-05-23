//! In HLSL Shader Model 5 (and earlier), a gradient-emitting texture sample
//! (`Sample`, `SampleBias`, `SampleCmp`) is illegal inside a loop whose iteration
//! count is non-uniform — and the compiler refuses to unroll loops whose worst-case
//! bound is too large (error X3511).
//!
//! SPIRV-Cross emits `OpImageSampleImplicitLod` (implicit gradient) from
//! `texture(s, uv)` calls. This pass walks the SPIR-V CFG, finds every
//! `OpImageSampleImplicitLod` (and the Sparse counterpart) that lives
//! inside a **non-uniform** structured loop, and rewrites it to
//! `OpImageSampleExplicitLod` with `Lod = 0`. The resulting HLSL emits
//! `SampleLevel(s, uv, 0)`, which carries an explicit LOD and is therefore
//! legal in a dynamic loop under FXC.
//!
//! "Non-uniform" here means the loop's termination condition transitively
//! depends on a value sourced from an `Input` storage variable (interpolated
//! vertex output / fragment-stage input), a previous texture sample, or a
//! derivative op. Loops bounded by constants or uniform/push-constant values
//! are left alone.
//!
//! Downgrading these samples inside non-uniform loops lose mipmap selection on
//! this path. Practically this affects only previous-pass outputs (rarely
//! mipmapped) and LUTs (almost never sampled inside a loop), so the visual
//! impact is negligible — and strictly better than the shader failing to
//! compile.

use rspirv::dr::{Builder, Function, Instruction, Operand};
use rustc_hash::{FxHashMap, FxHashSet};
use spirv::{ImageOperands, Op, StorageClass, Word};

pub struct LowerLoopSampleLod<'a> {
    pub builder: &'a mut Builder,
}

impl<'a> LowerLoopSampleLod<'a> {
    pub fn new(builder: &'a mut Builder) -> Self {
        Self { builder }
    }

    pub fn do_pass(&mut self) {
        // 1. Build a set of SSA value ids and variable ids that hold
        //    non-uniform data (per-fragment varyings, sample/derivative
        //    results, etc).
        let non_uniform = self.collect_non_uniform_ids();

        // 2. Find the blocks that sit inside a structured loop whose
        //    iteration count is non-uniform. Loops with uniform bounds
        //    are skipped — FXC handles them fine and we'd rather not
        //    lose mipmap selection unnecessarily.
        let in_loop = self.collect_in_nonuniform_loop_blocks(&non_uniform);
        if in_loop.is_empty() {
            return;
        }

        // 3. Identify candidate ImplicitLod samples we can safely lower.
        //    Collect indices first so we can mutate the module afterwards
        //    without invalidating iteration.
        let mut rewrites: Vec<(usize, usize, usize)> = Vec::new();
        for (fi, function) in self.builder.module_ref().functions.iter().enumerate() {
            for (bi, block) in function.blocks.iter().enumerate() {
                let Some(label_id) = block.label_id() else {
                    continue;
                };
                if !in_loop.contains(&label_id) {
                    continue;
                }
                for (ii, instr) in block.instructions.iter().enumerate() {
                    if is_lowerable_implicit_sample(instr) {
                        rewrites.push((fi, bi, ii));
                    }
                }
            }
        }

        if rewrites.is_empty() {
            return;
        }

        // 4. Find or create the LOD constant, then rewrite the instructions.
        let float_type = self.find_or_make_float_type();
        let lod_zero = self.builder.constant_bit32(float_type, 0u32);
        for (fi, bi, ii) in rewrites {
            let instr = &mut self.builder.module_mut().functions[fi].blocks[bi].instructions[ii];
            convert_implicit_to_explicit(instr, lod_zero);
        }
    }

    /// Conservative non-uniformity dataflow.
    ///
    /// Starting from declared non-uniform sources (Input-class variables
    /// and texture/derivative ops), iterate to fixed-point propagating
    /// non-uniformness through OpLoad, OpStore, OpAccessChain, and any
    /// other op whose result-id consumes a non-uniform IdRef.
    ///
    /// We err on the side of marking too much — false positives only
    /// cost us the ability to skip a rewrite that was already safe, while
    /// false negatives would skip rewriting a loop that FXC can't compile.
    fn collect_non_uniform_ids(&self) -> FxHashSet<Word> {
        let module = self.builder.module_ref();
        let mut non_uniform: FxHashSet<Word> = FxHashSet::default();

        // Seed: every `Input`-class OpVariable is non-uniform (it's the
        // per-fragment interpolated value). OpVariables in other storage
        // classes default to uniform and only become non-uniform via a
        // non-uniform OpStore (handled in the propagation loop below).
        for instr in &module.types_global_values {
            if instr.class.opcode != Op::Variable {
                continue;
            }
            let Some(Operand::StorageClass(sc)) = instr.operands.first() else {
                continue;
            };
            if matches!(*sc, StorageClass::Input) {
                if let Some(id) = instr.result_id {
                    non_uniform.insert(id);
                }
            }
        }

        // Propagate.
        loop {
            let mut changed = false;

            for function in &module.functions {
                for block in &function.blocks {
                    for instr in &block.instructions {
                        // OpStore propagates non-uniformness from value
                        // into the pointer's underlying variable.
                        if instr.class.opcode == Op::Store {
                            let (Some(Operand::IdRef(ptr)), Some(Operand::IdRef(val))) =
                                (instr.operands.first(), instr.operands.get(1))
                            else {
                                continue;
                            };
                            if non_uniform.contains(val) {
                                // Mark the pointer itself plus the root
                                // variable it transitively points to.
                                if non_uniform.insert(*ptr) {
                                    changed = true;
                                }
                                if let Some(root) = root_variable(*ptr, module) {
                                    if non_uniform.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                            continue;
                        }

                        let Some(result_id) = instr.result_id else {
                            continue;
                        };
                        if non_uniform.contains(&result_id) {
                            continue;
                        }

                        // Texture / derivative ops produce per-thread
                        // values regardless of their operand uniformity.
                        if is_per_thread_op(instr.class.opcode) {
                            non_uniform.insert(result_id);
                            changed = true;
                            continue;
                        }

                        // OpLoad: result is non-uniform iff the pointer
                        // is (which captures the OpStore propagation
                        // we did above for local variables).
                        if instr.class.opcode == Op::Load {
                            if let Some(Operand::IdRef(ptr)) = instr.operands.first() {
                                if non_uniform.contains(ptr)
                                    || root_variable(*ptr, module)
                                        .is_some_and(|r| non_uniform.contains(&r))
                                {
                                    non_uniform.insert(result_id);
                                    changed = true;
                                }
                            }
                            continue;
                        }

                        // OpAccessChain: derived pointer inherits the
                        // base's status, but also becomes non-uniform if
                        // any of the index operands are.
                        if matches!(
                            instr.class.opcode,
                            Op::AccessChain
                                | Op::InBoundsAccessChain
                                | Op::PtrAccessChain
                                | Op::InBoundsPtrAccessChain
                        ) {
                            let mut nu = false;
                            for op in &instr.operands {
                                if let Operand::IdRef(id) = op {
                                    if non_uniform.contains(id) {
                                        nu = true;
                                        break;
                                    }
                                }
                            }
                            if nu {
                                non_uniform.insert(result_id);
                                changed = true;
                            }
                            continue;
                        }

                        // OpPhi: non-uniform iff any incoming value is.
                        if instr.class.opcode == Op::Phi {
                            // Operands alternate IdRef(value), IdRef(label).
                            let mut nu = false;
                            for chunk in instr.operands.chunks(2) {
                                if let Some(Operand::IdRef(v)) = chunk.first() {
                                    if non_uniform.contains(v) {
                                        nu = true;
                                        break;
                                    }
                                }
                            }
                            if nu {
                                non_uniform.insert(result_id);
                                changed = true;
                            }
                            continue;
                        }

                        // Generic: result is non-uniform iff any IdRef
                        // operand is non-uniform.
                        for op in &instr.operands {
                            if let Operand::IdRef(id) = op {
                                if non_uniform.contains(id) {
                                    non_uniform.insert(result_id);
                                    changed = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if !changed {
                break;
            }
        }

        non_uniform
    }

    /// For every `OpLoopMerge`, check whether the loop's termination
    /// condition is non-uniform. If so, BFS the loop body (header to
    /// continue, stopping at merge) and add every block to the result
    /// set. Uniform loops are skipped entirely.
    fn collect_in_nonuniform_loop_blocks(&self, non_uniform: &FxHashSet<Word>) -> FxHashSet<Word> {
        let mut in_loop: FxHashSet<Word> = FxHashSet::default();

        for function in &self.builder.module_ref().functions {
            let mut label_to_idx: FxHashMap<Word, usize> = FxHashMap::default();
            for (bi, block) in function.blocks.iter().enumerate() {
                if let Some(id) = block.label_id() {
                    label_to_idx.insert(id, bi);
                }
            }

            for block in &function.blocks {
                let Some(header_id) = block.label_id() else {
                    continue;
                };
                let Some(loop_merge) = block
                    .instructions
                    .iter()
                    .find(|i| i.class.opcode == Op::LoopMerge)
                else {
                    continue;
                };
                let Some(Operand::IdRef(merge_id)) = loop_merge.operands.first().cloned() else {
                    continue;
                };

                // Pull the iteration condition out of the header (or the
                // first non-trivial successor, since SPIRV-Cross often
                // emits `header -> test -> body/merge` for `for`/`while`).
                if !loop_condition_is_non_uniform(function, block, merge_id, non_uniform) {
                    continue;
                }

                // BFS body, treating merge_id as a sink.
                let mut stack: Vec<Word> = vec![header_id];
                while let Some(label) = stack.pop() {
                    if label == merge_id {
                        continue;
                    }
                    if !in_loop.insert(label) {
                        continue;
                    }
                    let Some(&idx) = label_to_idx.get(&label) else {
                        continue;
                    };
                    let succ_block = &function.blocks[idx];
                    for succ in successor_labels(succ_block) {
                        if succ != merge_id {
                            stack.push(succ);
                        }
                    }
                }
            }
        }

        in_loop
    }

    fn find_or_make_float_type(&mut self) -> Word {
        for instr in &self.builder.module_ref().types_global_values {
            if instr.class.opcode != Op::TypeFloat {
                continue;
            }
            if let Some(Operand::LiteralBit32(32)) = instr.operands.first() {
                if let Some(id) = instr.result_id {
                    return id;
                }
            }
        }
        self.builder.type_float(32, None)
    }
}

/// Resolve a pointer SSA id back to the `OpVariable` it ultimately points
/// at, walking through `OpAccessChain` family ops. Returns `None` for
/// pointers we can't trace (e.g. function parameters, dynamically computed
/// pointers, etc.).
fn root_variable(mut id: Word, module: &rspirv::dr::Module) -> Option<Word> {
    // Bound iterations to defend against cycles in malformed modules.
    for _ in 0..32 {
        // Top-level OpVariable lives in `types_global_values`.
        if module
            .types_global_values
            .iter()
            .any(|i| i.result_id == Some(id) && i.class.opcode == Op::Variable)
        {
            return Some(id);
        }
        // Function-local OpVariable lives at the head of a function's first block.
        let mut found: Option<&Instruction> = None;
        for function in &module.functions {
            for block in &function.blocks {
                for instr in &block.instructions {
                    if instr.result_id == Some(id) {
                        found = Some(instr);
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        let instr = found?;
        if instr.class.opcode == Op::Variable {
            return Some(id);
        }
        if matches!(
            instr.class.opcode,
            Op::AccessChain
                | Op::InBoundsAccessChain
                | Op::PtrAccessChain
                | Op::InBoundsPtrAccessChain
                | Op::CopyObject
        ) {
            let Some(Operand::IdRef(base)) = instr.operands.first() else {
                return None;
            };
            id = *base;
            continue;
        }
        return None;
    }
    None
}

/// Determine whether the loop opened by `OpLoopMerge` in `header_block`
/// has a non-uniform termination condition. We trace the conditional
/// branch (`OpBranchConditional` or `OpSwitch`) terminating the header
/// or its single fall-through successor.
fn loop_condition_is_non_uniform(
    function: &Function,
    header_block: &rspirv::dr::Block,
    merge_id: Word,
    non_uniform: &FxHashSet<Word>,
) -> bool {
    // Candidates for the block that holds the iteration test: the header
    // itself, or — if the header just branches unconditionally — the
    // single successor (the typical `for`-loop shape SPIRV-Cross emits).
    let mut candidates: Vec<&rspirv::dr::Block> = Vec::with_capacity(2);
    candidates.push(header_block);
    if let Some(term) = header_block.instructions.last() {
        if term.class.opcode == Op::Branch {
            if let Some(Operand::IdRef(t)) = term.operands.first() {
                if let Some(target) = function.blocks.iter().find(|b| b.label_id() == Some(*t)) {
                    candidates.push(target);
                }
            }
        }
    }

    for candidate in candidates {
        let Some(term) = candidate.instructions.last() else {
            continue;
        };
        match term.class.opcode {
            Op::BranchConditional => {
                // Only treat this as the loop's exit test if one of the
                // targets is the merge block — otherwise it's a nested
                // `if` inside the loop body, which doesn't drive the
                // iteration count.
                let mut targets = term.operands.iter().filter_map(|o| {
                    if let Operand::IdRef(t) = o {
                        Some(*t)
                    } else {
                        None
                    }
                });
                let cond = targets.next();
                let exits_loop = targets.any(|t| t == merge_id);
                if !exits_loop {
                    continue;
                }
                if let Some(cond_id) = cond {
                    if non_uniform.contains(&cond_id) {
                        return true;
                    }
                }
            }
            Op::Switch => {
                // Switch selector is the first IdRef operand.
                let mut targets_after_default = term.operands.iter().skip(2).filter_map(|o| {
                    if let Operand::IdRef(t) = o {
                        Some(*t)
                    } else {
                        None
                    }
                });
                let exits_loop = targets_after_default.any(|t| t == merge_id);
                if !exits_loop {
                    continue;
                }
                if let Some(Operand::IdRef(sel)) = term.operands.first() {
                    if non_uniform.contains(sel) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }

    false
}

/// Collect the labels of all successor blocks of `block` from its terminator.
fn successor_labels(block: &rspirv::dr::Block) -> Vec<Word> {
    let mut out = Vec::new();
    let Some(term) = block.instructions.last() else {
        return out;
    };
    match term.class.opcode {
        Op::Branch => {
            if let Some(Operand::IdRef(t)) = term.operands.first() {
                out.push(*t);
            }
        }
        Op::BranchConditional => {
            for operand in term.operands.iter().take(3).skip(1) {
                if let Operand::IdRef(t) = operand {
                    out.push(*t);
                }
            }
        }
        Op::Switch => {
            for op in &term.operands {
                if let Operand::IdRef(t) = op {
                    out.push(*t);
                }
            }
        }
        _ => {}
    }
    out
}

/// Whether `instr` is an implicit-LOD sample whose existing image-operands
/// (if any) are all compatible with rewriting to ExplicitLod with LOD 0.
/// We refuse to touch samples that already carry `Bias` (incompatible
/// with ExplicitLod) or `Lod`/`Grad` (already explicit).
fn is_lowerable_implicit_sample(instr: &Instruction) -> bool {
    match instr.class.opcode {
        Op::ImageSampleImplicitLod | Op::ImageSparseSampleImplicitLod => {}
        _ => return false,
    }
    if let Some(Operand::ImageOperands(existing)) = instr.operands.get(2) {
        if existing.contains(ImageOperands::BIAS)
            || existing.contains(ImageOperands::LOD)
            || existing.contains(ImageOperands::GRAD)
        {
            return false;
        }
    }
    true
}

/// Op codes whose result is treated as non-uniform regardless of operand
/// uniformity (texture reads, derivatives, fragment-stage helpers).
fn is_per_thread_op(op: Op) -> bool {
    matches!(
        op,
        Op::ImageSampleImplicitLod
            | Op::ImageSampleExplicitLod
            | Op::ImageSampleDrefImplicitLod
            | Op::ImageSampleDrefExplicitLod
            | Op::ImageSampleProjImplicitLod
            | Op::ImageSampleProjExplicitLod
            | Op::ImageSampleProjDrefImplicitLod
            | Op::ImageSampleProjDrefExplicitLod
            | Op::ImageRead
            | Op::ImageFetch
            | Op::ImageGather
            | Op::ImageDrefGather
            | Op::ImageSparseSampleImplicitLod
            | Op::ImageSparseSampleExplicitLod
            | Op::ImageSparseSampleDrefImplicitLod
            | Op::ImageSparseSampleDrefExplicitLod
            | Op::ImageSparseRead
            | Op::ImageSparseFetch
            | Op::ImageSparseGather
            | Op::ImageSparseDrefGather
            | Op::DPdx
            | Op::DPdy
            | Op::DPdxFine
            | Op::DPdyFine
            | Op::DPdxCoarse
            | Op::DPdyCoarse
            | Op::Fwidth
            | Op::FwidthFine
            | Op::FwidthCoarse
    )
}

/// Convert an OpImageSampleImplicitLod (or sparse variant) into the
/// corresponding ExplicitLod with `Lod = lod_zero`. Preserves any other
/// image operands (e.g. ConstOffset) that came along on the original.
fn convert_implicit_to_explicit(instr: &mut Instruction, lod_zero: Word) {
    match instr.class.opcode {
        Op::ImageSampleImplicitLod => {
            instr.class = rspirv::grammar::INSTRUCTION_TABLE.get(Op::ImageSampleExplicitLod);
        }
        Op::ImageSparseSampleImplicitLod => {
            instr.class = rspirv::grammar::INSTRUCTION_TABLE.get(Op::ImageSparseSampleExplicitLod);
        }
        _ => return,
    }

    let lod_bit = ImageOperands::LOD;
    if let Some(Operand::ImageOperands(mask)) = instr.operands.get_mut(2) {
        *mask |= lod_bit;
        instr.operands.insert(3, Operand::IdRef(lod_zero));
    } else {
        instr.operands.push(Operand::ImageOperands(lod_bit));
        instr.operands.push(Operand::IdRef(lod_zero));
    }
}
