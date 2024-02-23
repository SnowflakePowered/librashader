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
                context.named_buffer_data_size(buffer, buffer_size, glow::STREAM_DRAW);
                ring.items_mut()[i] = buffer;
            }
        }

        Gl46UboRing { ring }
    }

    fn bind_for_frame(
        &mut self,
        context: &glow::Context,
        ubo: &BufferReflection<u32>,
        ubo_location: &UniformLocation<Option<u32>>,
        storage: &impl UniformStorageAccess,
    ) {
        let size = ubo.size;
        let buffer = self.ring.current();

        unsafe {
            gl::NamedBufferSubData(*buffer, 0, size as GLsizeiptr, storage.ubo_pointer().cast());

            if ubo_location.vertex != gl::INVALID_INDEX {
                gl::BindBufferBase(gl::UNIFORM_BUFFER, ubo_location.vertex, *buffer);
            }
            if ubo_location.fragment != gl::INVALID_INDEX {
                gl::BindBufferBase(gl::UNIFORM_BUFFER, ubo_location.fragment, *buffer);
            }
        }
        self.ring.next()
    }
}
