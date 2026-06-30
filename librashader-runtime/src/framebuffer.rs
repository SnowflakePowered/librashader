use crate::binding::{BindingRequirements, BindingUtil};
use bit_set::BitSet;
use librashader_common::map::FastHashMap;
use librashader_common::Size;
use librashader_reflect::reflect::semantics::BindingMeta;
use std::collections::VecDeque;
use std::ops::{Index, IndexMut};
use num_traits::AsPrimitive;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Slot(i32);

impl Slot {
    pub const NONE: Slot = Slot(-1);

    const fn new(index: usize) -> Slot {
        Slot(index as i32)
    }

    /// The pool index, or `None` if this is [`Slot::NONE`].
    const fn get(self) -> Option<usize> {
        if self.0 < 0 {
            None
        } else {
            Some(self.0 as usize)
        }
    }
}

/// A pool of framebuffers with internal liveness analysis, indexed by pass number.
pub struct FramebufferPool<F> {
    pool: Box<[F]>,
    slots: Box<[Slot]>,

    // At pass i, store the last pass that reads i (the liveness period). Empty when init for feedback.
    last_use: Box<[usize]>,
}

impl<F> FramebufferPool<F> {
    /// Calculate liveness assignments based on pass sizes.
    pub(crate) fn prepare(&mut self, sizes: &[Size<u32>]) {
        struct Event {
            slot: usize,
            size: u64,
            next_slot: usize,
        }

        let n = sizes.len();

        // `free[size]` holds slots currently free and of that size.
        let mut free: FastHashMap<u64, Vec<usize>> = FastHashMap::default();

        let mut freed_at = vec![usize::MAX; n + 1];
        let mut events: Vec<Event> = Vec::with_capacity(n);
        let mut slot_count = 0;

        for pass in 0..n {
            let mut event = freed_at[pass];
            while event != usize::MAX {
                let Event { slot, size, next_slot } = events[event];
                free.entry(size).or_default().push(slot);
                event = next_slot;
            }

            let size = sizes[pass].as_();
            let slot = free.get_mut(&size).and_then(Vec::pop).unwrap_or_else(|| {
                let slot = slot_count;
                slot_count += 1;
                slot
            });

            self.slots[pass] = Slot::new(slot);
            let free_at = self.last_use[pass] + 1;
            events.push(Event { slot, size, next_slot: freed_at[free_at] });
            freed_at[free_at] = events.len() - 1;
        }
    }

    /// Returns whether the given pass maps to a buffer.
    pub fn contains(&self, pass: usize) -> bool {
        self.slots.get(pass).and_then(|s| s.get()).is_some()
    }
}

impl<F> Index<usize> for FramebufferPool<F> {
    type Output = F;

    fn index(&self, pass: usize) -> &F {
        &self.pool[self.slots[pass].get().expect("pass has no framebuffer")]
    }
}

impl<F> IndexMut<usize> for FramebufferPool<F> {
    fn index_mut(&mut self, pass: usize) -> &mut F {
        &mut self.pool[self.slots[pass].get().expect("pass has no framebuffer")]
    }
}

/// Helper to initialize framebuffers in a graphics API agnostic way.
pub struct FramebufferInit<'a, F, I, E> {
    owned_generator: &'a dyn Fn() -> Result<F, E>,
    input_generator: &'a dyn Fn() -> I,
    requirements: BindingRequirements,
    filters_count: usize,
}

impl<'a, F, I, E> FramebufferInit<'a, F, I, E> {
    /// Create a new framebuffer initializer with the given
    /// closures to create owned framebuffers and image views.
    pub fn new(
        filters: impl Iterator<Item = &'a BindingMeta> + ExactSizeIterator,
        owned_generator: &'a dyn Fn() -> Result<F, E>,
        input_generator: &'a dyn Fn() -> I,
    ) -> Self {
        let filters_count = filters.len();
        let requirements = BindingMeta::calculate_requirements(filters);

        Self {
            owned_generator,
            input_generator,
            filters_count,
            requirements,
        }
    }

    /// Initialize history framebuffers and views.
    pub fn init_history(&self) -> Result<(VecDeque<F>, Box<[I]>), E> {
        init_history(
            self.requirements.required_history,
            self.owned_generator,
            self.input_generator,
        )
    }

    /// Initialize output framebuffers pooled by pass-output liveness.
    pub fn init_output_framebuffers(&self) -> Result<(FramebufferPool<F>, Box<[I]>), E> {
        init_output_framebuffers(
            self.filters_count,
            &self.requirements.last_use,
            self.owned_generator,
            self.input_generator,
        )
    }

    /// Initialize sparse feedback framebuffers, allocating a previous-frame copy only for
    /// passes referenced as `PassFeedback`.
    pub fn init_feedback_framebuffers(&self) -> Result<(FramebufferPool<F>, Box<[I]>), E> {
        init_feedback_framebuffers(
            self.filters_count,
            &self.requirements.feedback_mask,
            self.owned_generator,
            self.input_generator,
        )
    }

    /// Get if the final pass is used as feedback.
    pub const fn uses_final_pass_as_feedback(&self) -> bool {
        self.requirements.uses_final_pass_as_feedback
    }
}

fn init_history<'a, F, I, E>(
    required_images: usize,
    owned_generator: impl Fn() -> Result<F, E>,
    input_generator: impl Fn() -> I,
) -> Result<(VecDeque<F>, Box<[I]>), E> {
    // Since OriginalHistory0 aliases source, it always gets bound if present, and we don't need to
    // store it. However, if even OriginalHistory1 is used, then we need to store it, hence we check if
    // required_images is less than 1, and only then do we return an empty history queue.
    if required_images < 1 {
        return Ok((VecDeque::new(), Box::new([])));
    }

    let mut framebuffers = VecDeque::with_capacity(required_images);
    framebuffers.resize_with(required_images, owned_generator);

    let framebuffers = framebuffers
        .into_iter()
        .collect::<Result<VecDeque<F>, E>>()?;

    let mut history_textures = Vec::new();
    history_textures.resize_with(required_images, input_generator);

    Ok((framebuffers, history_textures.into_boxed_slice()))
}

fn init_output_framebuffers<F, I, E>(
    filters_count: usize,
    last_use: &[usize],
    owned_generator: impl Fn() -> Result<F, E>,
    input_generator: impl Fn() -> I,
) -> Result<(FramebufferPool<F>, Box<[I]>), E> {
    let mut pool = Vec::with_capacity(filters_count);
    pool.resize_with(filters_count, &owned_generator);
    let pool = pool
        .into_iter()
        .collect::<Result<Vec<F>, E>>()?
        .into_boxed_slice();

    let mut textures = Vec::new();
    textures.resize_with(filters_count, input_generator);

    Ok((
        FramebufferPool {
            pool,
            last_use: last_use.to_vec().into_boxed_slice(),
            slots: (0..filters_count).map(Slot::new).collect(),
        },
        textures.into_boxed_slice(),
    ))
}

fn init_feedback_framebuffers<F, I, E>(
    filters_count: usize,
    feedback_mask: &BitSet,
    owned_generator: impl Fn() -> Result<F, E>,
    input_generator: impl Fn() -> I,
) -> Result<(FramebufferPool<F>, Box<[I]>), E> {

    // assign feedback slots according to the usage mask
    fn assign_slots(mask: &BitSet, filters_count: usize) -> (usize, Box<[Slot]>) {
        let mut slot_of_pass = vec![Slot::NONE; filters_count];
        let mut count = 0;
        for pass in mask.iter() {
            if pass < filters_count {
                slot_of_pass[pass] = Slot::new(count);
                count += 1;
            }
        }
        (count, slot_of_pass.into_boxed_slice())
    }

    let (count, slots) = assign_slots(feedback_mask, filters_count);

    let mut pool = Vec::with_capacity(count);
    pool.resize_with(count, owned_generator);
    let pool = pool
        .into_iter()
        .collect::<Result<Vec<F>, E>>()?
        .into_boxed_slice();

    let mut textures = Vec::new();
    textures.resize_with(filters_count, input_generator);

    Ok((
        FramebufferPool {
            pool,
            last_use: Box::new([]),
            slots,
        },
        textures.into_boxed_slice(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{FramebufferPool, Slot};
    use librashader_common::Size;
    use std::collections::HashSet;

    fn fb(last_use: Vec<usize>) -> FramebufferPool<()> {
        let n = last_use.len();
        FramebufferPool {
            pool: vec![(); n].into_boxed_slice(),
            last_use: last_use.into_boxed_slice(),
            slots: vec![Slot::new(0); n].into_boxed_slice(),
        }
    }

    fn distinct(slots: &[Slot]) -> usize {
        slots.iter().copied().collect::<HashSet<_>>().len()
    }

    #[test]
    fn liveness_pools_chain() {
        let size = |w, h| Size::<u32>::new(w, h);
        let last_use = vec![1, 2, 10, 4, 5, 6, 7, 8, 9, 10, 10];
        let sizes = vec![
            size(1280, 960),  // p0
            size(1280, 960),  // p1
            size(1280, 3360), // p2 (retained, read by p3..p10)
            size(1280, 1680), // p3
            size(1280, 1680), // p4
            size(1280, 1680), // p5
            size(1280, 1680), // p6
            size(1280, 1680), // p7
            size(1280, 1680), // p8
            size(3840, 1680), // p9
            size(1280, 720),  // p10 (final / viewport)
        ];

        let mut output = fb(last_use);
        output.prepare(&sizes);

        // p3..=p8 (six same-size passes, each live only into the next) collapse onto two
        // ping-pong buffers.
        assert_eq!(distinct(&output.slots[3..=8]), 2);

        // The whole chain needs 7 physical buffers instead of 11, and no buffer is ever
        // assigned two different sizes (the invariant that keeps it deferred-safe).
        assert_eq!(distinct(&output.slots), 7);
        let mut seen: std::collections::HashMap<Slot, Size<u32>> = Default::default();
        for (pass, &slot) in output.slots.iter().enumerate() {
            assert_eq!(*seen.entry(slot).or_insert(sizes[pass]), sizes[pass]);
        }
    }

    // A uniform-size linear chain (the common case) collapses to a 2-buffer ping-pong.
    #[test]
    fn liveness_pools_uniform_chain() {
        let sizes = vec![Size::<u32>::new(1920, 1080); 8];
        let mut output = fb(vec![1, 2, 3, 4, 5, 6, 7, 7]);
        output.prepare(&sizes);
        assert_eq!(distinct(&output.slots), 2);
    }
}
