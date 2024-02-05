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

pub struct Gl3UboRing<const SIZE: usize> {
    ring: InlineRingBuffer<glow::Buffer, SIZE>,
}

impl<const SIZE: usize> Gl3UboRing<SIZE> {
    const _ASSERT_TRANSMUTABLE: () = assert!(
        std::mem::size_of::<[glow::Buffer; SIZE]>()
            == std::mem::size_of::<[MaybeUninit<glow::Buffer>; SIZE]>()
    );
}

impl<const SIZE: usize> UboRing<SIZE> for Gl3UboRing<SIZE> {
    fn new(ctx: &glow::Context, buffer_size: u32) -> error::Result<Self> {
        // TODO: array::try_from_fn whenever that gets stabilized
        //       this is basically blocking on try_trait_v2
        let mut items: [MaybeUninit<glow::Buffer>; SIZE] = [MaybeUninit::zeroed(); SIZE];
        for items in items.iter_mut() {
            unsafe {
                let buffer = ctx
                    .create_buffer()
                    .map(|buffer| {
                        ctx.bind_buffer(glow::UNIFORM_BUFFER, Some(buffer));
                        ctx.buffer_data_size(
                            glow::UNIFORM_BUFFER,
                            buffer_size as i32,
                            glow::STREAM_DRAW,
                        );
                        ctx.bind_buffer(glow::UNIFORM_BUFFER, None);
                        buffer
                    })
                    .map_err(FilterChainError::GlError)?;

                *items = MaybeUninit::new(buffer)
            }
        }

        // SAFETY: everything was initialized above.
        // MaybeUninit<glow::Buffer> and glow::Buffer have the same size.
        let items: [glow::Buffer; SIZE] = unsafe { std::mem::transmute_copy(&items) };

        let ring: InlineRingBuffer<glow::Buffer, SIZE> = InlineRingBuffer::from_array(items);

        Ok(Gl3UboRing { ring })
    }

    fn bind_for_frame(
        &mut self,
        ctx: &glow::Context,
        ubo: &BufferReflection<u32>,
        ubo_location: &UniformLocation<Option<u32>>,
        storage: &impl UniformStorageAccess,
    ) {
        let buffer = *self.ring.current();

        unsafe {
            ctx.bind_buffer(glow::UNIFORM_BUFFER, Some(buffer));
            ctx.buffer_sub_data_u8_slice(
                glow::UNIFORM_BUFFER,
                0,
                &storage.ubo_slice()[0..ubo.size as usize],
            );
            ctx.bind_buffer(glow::UNIFORM_BUFFER, None);

            if let Some(vertex) = ubo_location
                .vertex
                .filter(|vertex| *vertex != glow::INVALID_INDEX)
            {
                ctx.bind_buffer_base(glow::UNIFORM_BUFFER, vertex, Some(buffer));
            }
            if let Some(fragment) = ubo_location
                .fragment
                .filter(|fragment| *fragment != glow::INVALID_INDEX)
            {
                ctx.bind_buffer_base(glow::UNIFORM_BUFFER, fragment, Some(buffer));
            }
        }
        self.ring.next()
    }
}
