use crate::binding::UniformLocation;
use crate::error;
use crate::error::FilterChainError;
use crate::gl::UboRing;
use glow::HasContext;
use librashader_reflect::reflect::semantics::BufferReflection;
use librashader_runtime::ringbuffer::InlineRingBuffer;
use librashader_runtime::ringbuffer::RingBuffer;
use librashader_runtime::uniforms::UniformStorageAccess;
use std::mem::MaybeUninit;

pub struct Gl46UboRing<const SIZE: usize> {
    ring: InlineRingBuffer<glow::Buffer, SIZE>,
}

impl<const SIZE: usize> Gl46UboRing<SIZE> {
    const _ASSERT_TRANSMUTABLE: () = assert!(
        std::mem::size_of::<[glow::Buffer; SIZE]>()
            == std::mem::size_of::<[MaybeUninit<glow::Buffer>; SIZE]>()
    );
}

impl<const SIZE: usize> UboRing<SIZE> for Gl46UboRing<SIZE> {
    fn new(context: &glow::Context, buffer_size: u32) -> error::Result<Self> {
        // TODO: array::try_from_fn whenever that gets stabilized
        //       this is basically blocking on try_trait_v2
        let mut items: [MaybeUninit<glow::Buffer>; SIZE] = [MaybeUninit::zeroed(); SIZE];
        for items in items.iter_mut() {
            unsafe {
                let buffer = context
                    .create_named_buffer()
                    .map(|buffer| {
                        context.named_buffer_data_size(
                            buffer,
                            buffer_size as i32,
                            glow::STREAM_DRAW,
                        );
                        buffer
                    })
                    .map_err(FilterChainError::GlError)?;

                *items = MaybeUninit::new(buffer)
            }
        }

        // SAFETY: everything was initialized above.
        let items: [glow::Buffer; SIZE] = unsafe { std::mem::transmute_copy(&items) };
        let ring: InlineRingBuffer<glow::Buffer, SIZE> = InlineRingBuffer::from_array(items);

        Ok(Gl46UboRing { ring })
    }

    fn bind_for_frame(
        &mut self,
        context: &glow::Context,
        ubo: &BufferReflection<u32>,
        ubo_location: &UniformLocation<Option<u32>>,
        storage: &impl UniformStorageAccess,
    ) {
        let buffer = *self.ring.current();

        unsafe {
            context.named_buffer_sub_data_u8_slice(
                buffer,
                0,
                &storage.ubo_slice()[0..ubo.size as usize],
            );

            if let Some(vertex) = ubo_location
                .vertex
                .filter(|vertex| *vertex != glow::INVALID_INDEX)
            {
                context.bind_buffer_base(glow::UNIFORM_BUFFER, vertex, Some(buffer));
            }
            if let Some(fragment) = ubo_location
                .fragment
                .filter(|fragment| *fragment != glow::INVALID_INDEX)
            {
                context.bind_buffer_base(glow::UNIFORM_BUFFER, fragment, Some(buffer));
            }
        }
        self.ring.next()
    }
}
