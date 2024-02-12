use crate::error;
use crate::error::FilterChainError;
use icrate::Foundation::NSRange;
use icrate::Metal::{
    MTLBuffer, MTLDevice, MTLResourceStorageModeManaged, MTLResourceStorageModeShared,
};
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use std::ops::{Deref, DerefMut};

pub struct MetalBuffer {
    buffer: Id<ProtocolObject<dyn MTLBuffer>>,
    size: usize,
}

impl AsRef<ProtocolObject<dyn MTLBuffer>> for MetalBuffer {
    fn as_ref(&self) -> &ProtocolObject<dyn MTLBuffer> {
        self.buffer.as_ref()
    }
}

impl MetalBuffer {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>, size: usize) -> error::Result<Self> {
        let resource_mode = if cfg!(target_os = "ios") {
            MTLResourceStorageModeShared
        } else {
            MTLResourceStorageModeManaged
        };

        let buffer = device
            .newBufferWithLength_options(size, resource_mode)
            .ok_or(FilterChainError::BufferError)?;
        Ok(Self { buffer, size })
    }

    pub fn flush(&self) {
        // We don't know what was actually written to so...
        self.buffer.didModifyRange(NSRange {
            location: 0,
            length: self.size,
        })
    }
}

impl Deref for MetalBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        // SAFETY: the lifetime of this reference must be longer than of the MetalBuffer.
        // Additionally, `MetalBuffer.buffer` is never lent out directly
        unsafe { std::slice::from_raw_parts(self.buffer.contents().as_ptr().cast(), self.size) }
    }
}

impl DerefMut for MetalBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: the lifetime of this reference must be longer than of the MetalBuffer.
        // Additionally, `MetalBuffer.buffer` is never lent out directly
        unsafe { std::slice::from_raw_parts_mut(self.buffer.contents().as_ptr().cast(), self.size) }
    }
}
