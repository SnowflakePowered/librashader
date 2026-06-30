use crate::binding::{BindingRequirements, BindingUtil};
use bit_set::BitSet;
use librashader_reflect::reflect::semantics::BindingMeta;
use std::collections::VecDeque;
use std::ops::{Index, IndexMut};

pub const IMMEDIATE_POOL_SIZE: usize = 2;

/// An index into a physical framebuffer array — the immediate pool, the retained images, or
/// the feedback copies — as distinct from a pass index.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Slot(usize);

/// How long a pass output is stored for the current frame.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OutputLifetime {
    /// Pass output only needs to live until after the next pass, and is stored in the
    /// immediate pool slot at this index.
    Immediate(Slot),
    /// Pass output is referenced as feedback or `PassOutput`, and is stored in the retained
    /// image at this index.
    Retained(Slot),
}

/// Output framebuffers with a routing table that maps each pass to its physical storage for
/// the frame. Index by pass number to obtain its framebuffer.
pub struct OutputFramebuffers<F> {
    immediates: Box<[F]>,
    retained: Box<[F]>,
    routing: Box<[OutputLifetime]>,
}

impl<F> Index<usize> for OutputFramebuffers<F> {
    type Output = F;

    fn index(&self, pass: usize) -> &F {
        match self.routing[pass] {
            OutputLifetime::Immediate(slot) => &self.immediates[slot.0],
            OutputLifetime::Retained(slot) => &self.retained[slot.0],
        }
    }
}

impl<F> IndexMut<usize> for OutputFramebuffers<F> {
    fn index_mut(&mut self, pass: usize) -> &mut F {
        match self.routing[pass] {
            OutputLifetime::Immediate(slot) => &mut self.immediates[slot.0],
            OutputLifetime::Retained(slot) => &mut self.retained[slot.0],
        }
    }
}

/// Previous-frame copies kept only for passes referenced as `PassFeedback`. Index by pass
/// number to obtain its framebuffer; [`FeedbackFramebuffers::contains`] reports whether a
/// pass has one.
pub struct FeedbackFramebuffers<F> {
    framebuffers: Box<[F]>,
    slot_of_pass: Box<[Option<Slot>]>,
}

impl<F> FeedbackFramebuffers<F> {
    /// Returns whether the given pass is referenced as feedback.
    pub fn contains(&self, pass: usize) -> bool {
        self.slot_of_pass.get(pass).copied().flatten().is_some()
    }
}

impl<F> Index<usize> for FeedbackFramebuffers<F> {
    type Output = F;

    fn index(&self, pass: usize) -> &F {
        let slot = self.slot_of_pass[pass].expect("pass is not referenced as feedback");
        &self.framebuffers[slot.0]
    }
}

impl<F> IndexMut<usize> for FeedbackFramebuffers<F> {
    fn index_mut(&mut self, pass: usize) -> &mut F {
        let slot = self.slot_of_pass[pass].expect("pass is not referenced as feedback");
        &mut self.framebuffers[slot.0]
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

    /// Initialize output framebuffers and views.
    pub fn init_output_framebuffers(&self) -> Result<(Box<[F]>, Box<[I]>), E> {
        init_output_framebuffers(
            self.filters_count,
            self.owned_generator,
            self.input_generator,
        )
    }

    /// Initialize output framebuffers.
    pub fn init_pooled_output_framebuffers(
        &self,
    ) -> Result<(OutputFramebuffers<F>, Box<[I]>), E> {
        init_pooled_output_framebuffers(
            self.filters_count,
            &self.requirements.retained_output_mask,
            self.owned_generator,
            self.input_generator,
        )
    }

    /// Initialize sparse feedback framebuffers, allocating a previous-frame copy only for
    /// passes referenced as `PassFeedback`.
    pub fn init_feedback_framebuffers(&self) -> Result<(FeedbackFramebuffers<F>, Box<[I]>), E> {
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

/// Assign a dense slot to each in-range set bit of `mask`, in ascending pass order.
///
/// Returns the number of slots and a `pass index -> slot` table of length `filters_count`.
/// Out-of-range bits (>= `filters_count`, possible for the permissive final-pass feedback
/// case) are ignored so they neither consume a slot nor index past the framebuffer slice.
fn assign_slots(mask: &BitSet, filters_count: usize) -> (usize, Box<[Option<Slot>]>) {
    let mut slot_of_pass = vec![None; filters_count];
    let mut count = 0;
    for pass in mask.iter() {
        if pass < filters_count {
            slot_of_pass[pass] = Some(Slot(count));
            count += 1;
        }
    }
    (count, slot_of_pass.into_boxed_slice())
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
    len: usize,
    owned_generator: impl Fn() -> Result<F, E>,
    input_generator: impl Fn() -> I,
) -> Result<(Box<[F]>, Box<[I]>), E> {
    let mut output_framebuffers = Vec::new();
    output_framebuffers.resize_with(len, owned_generator);

    // resolve all results
    let output_framebuffers = output_framebuffers
        .into_iter()
        .collect::<Result<Vec<F>, E>>()?;

    let mut output_textures = Vec::new();
    output_textures.resize_with(len, input_generator);

    Ok((
        output_framebuffers.into_boxed_slice(),
        output_textures.into_boxed_slice(),
    ))
}

fn init_pooled_output_framebuffers<F, I, E>(
    filters_count: usize,
    retained_mask: &BitSet,
    owned_generator: impl Fn() -> Result<F, E>,
    input_generator: impl Fn() -> I,
) -> Result<(OutputFramebuffers<F>, Box<[I]>), E> {
    let mut routing = Vec::with_capacity(filters_count);
    let mut retained_count = 0;
    let mut pool_slot = 0;
    for pass in 0..filters_count {
        if retained_mask.contains(pass) {
            routing.push(OutputLifetime::Retained(Slot(retained_count)));
            retained_count += 1;
        } else {
            routing.push(OutputLifetime::Immediate(Slot(pool_slot)));
            pool_slot = (pool_slot + 1) % IMMEDIATE_POOL_SIZE;
        }
    }

    let mut immediates = Vec::with_capacity(IMMEDIATE_POOL_SIZE);
    immediates.resize_with(IMMEDIATE_POOL_SIZE, &owned_generator);
    let immediates = immediates
        .into_iter()
        .collect::<Result<Vec<F>, E>>()?
        .into_boxed_slice();

    let mut retained = Vec::with_capacity(retained_count);
    retained.resize_with(retained_count, &owned_generator);
    let retained = retained
        .into_iter()
        .collect::<Result<Vec<F>, E>>()?
        .into_boxed_slice();

    let mut textures = Vec::new();
    textures.resize_with(filters_count, input_generator);

    Ok((
        OutputFramebuffers {
            immediates,
            retained,
            routing: routing.into_boxed_slice(),
        },
        textures.into_boxed_slice(),
    ))
}

fn init_feedback_framebuffers<F, I, E>(
    filters_count: usize,
    feedback_mask: &BitSet,
    owned_generator: impl Fn() -> Result<F, E>,
    input_generator: impl Fn() -> I,
) -> Result<(FeedbackFramebuffers<F>, Box<[I]>), E> {
    let (count, slot_of_pass) = assign_slots(feedback_mask, filters_count);

    let mut framebuffers = Vec::with_capacity(count);
    framebuffers.resize_with(count, owned_generator);
    let framebuffers = framebuffers
        .into_iter()
        .collect::<Result<Vec<F>, E>>()?
        .into_boxed_slice();

    let mut textures = Vec::new();
    textures.resize_with(filters_count, input_generator);

    Ok((
        FeedbackFramebuffers {
            framebuffers,
            slot_of_pass,
        },
        textures.into_boxed_slice(),
    ))
}
