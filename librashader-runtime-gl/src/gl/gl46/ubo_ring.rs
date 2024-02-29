use glow::HasContext;
use crate::binding::UniformLocation;
use crate::gl::UboRing;
use librashader_reflect::reflect::semantics::BufferReflection;
use librashader_runtime::ringbuffer::InlineRingBuffer;
use librashader_runtime::ringbuffer::RingBuffer;
use librashader_runtime::uniforms::UniformStorageAccess;

pub struct Gl46UboRing<const SIZE: usize> {
    ring: InlineRingBuffer<glow::Buffer, SIZE>,
}

impl<const SIZE: usize> UboRing<SIZE> for Gl46UboRing<SIZE> {
    fn new(context: &glow::Context, buffer_size: u32) -> Self {
        let mut ring: InlineRingBuffer<glow::Buffer, SIZE> = InlineRingBuffer::new();

        for i in 0..SIZE {
            unsafe {
                let buffer = context.create_named_buffer()?;
                context.named_buffer_data_size(buffer, buffer_size as i32, glow::STREAM_DRAW);
                ring.items_mut()[i] = buffer;
            }
        }

        Gl46UboRing { ring }
    }

    fn bind_for_frame(
        &mut self,
        context: &glow::Context,
        _ubo: &BufferReflection<u32>,
        ubo_location: &UniformLocation<Option<u32>>,
        storage: &impl UniformStorageAccess,
    ) {
        let buffer = *self.ring.current();

        unsafe {
            context.named_buffer_sub_data_u8_slice(buffer, 0, storage.ubo_slice());

            if let Some(vertex) = ubo_location.vertex && vertex != glow::INVALID_INDEX {
                context.bind_buffer_base(glow::UNIFORM_BUFFER, vertex, Some(buffer));
            }
            if let Some(fragment) = ubo_location.fragment && fragment!= glow::INVALID_INDEX {
                context.bind_buffer_base(glow::UNIFORM_BUFFER, fragment, Some(buffer));
            }
        }
        self.ring.next()
    }
}
