use array_concat::concat_arrays;
use bytemuck::{Pod, Zeroable};
use icrate::Metal::{
    MTLBuffer, MTLDevice, MTLPrimitiveTypeTriangleStrip, MTLRenderCommandEncoder,
    MTLResourceStorageModeManaged, MTLResourceStorageModeShared,
};
use librashader_runtime::quad::QuadType;
use objc2::rc::Id;
use objc2::runtime::ProtocolObject;
use std::ffi::c_void;
use std::ptr::NonNull;

use crate::error::{FilterChainError, Result};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Zeroable, Pod)]
struct MetalVertex {
    position: [f32; 4],
    texcoord: [f32; 2],
}

const FINAL_VBO_DATA: [MetalVertex; 4] = [
    MetalVertex {
        position: [0.0, 1.0, 0.0, 1.0],
        texcoord: [0.0, 1.0],
    },
    MetalVertex {
        position: [1.0, 1.0, 0.0, 1.0],
        texcoord: [1.0, 1.0],
    },
    MetalVertex {
        position: [0.0, 0.0, 0.0, 1.0],
        texcoord: [0.0, 0.0],
    },
    MetalVertex {
        position: [1.0, 0.0, 0.0, 1.0],
        texcoord: [1.0, 0.0],
    },
];

#[rustfmt::skip]
const VBO_OFFSCREEN: [f32; 16] = [
    // Offscreen
    -1.0f32, -1.0, 0.0, 0.0,
    -1.0, 1.0, 0.0, 1.0,
    1.0, -1.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];

#[rustfmt::skip]
const VBO_DEFAULT_FINAL: [f32; 16] = [
    // Final
    0.0f32, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 1.0,
    1.0, 0.0, 1.0, 0.0,
    1.0, 1.0, 1.0, 1.0,
];

const VBO_DATA: [f32; 32] = concat_arrays!(VBO_OFFSCREEN, VBO_DEFAULT_FINAL);

pub struct DrawQuad {
    buffer: Id<ProtocolObject<dyn MTLBuffer>>,
}

impl DrawQuad {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<DrawQuad> {
        let vbo_data: &'static [u8] = bytemuck::cast_slice(&VBO_DATA);
        let Some(buffer) = (unsafe {
            device.newBufferWithBytes_length_options(
                // SAFETY: this pointer is const.
                // https://developer.apple.com/documentation/metal/mtldevice/1433429-newbufferwithbytes
                NonNull::new_unchecked(vbo_data.as_ptr() as *mut c_void),
                vbo_data.len(),
                if cfg!(target_os = "ios") {
                    MTLResourceStorageModeShared
                } else {
                    MTLResourceStorageModeManaged
                },
            )
        }) else {
            return Err(FilterChainError::BufferError);
        };

        Ok(DrawQuad { buffer })
    }

    pub fn draw_quad(&self, cmd: &ProtocolObject<dyn MTLRenderCommandEncoder>, vbo: QuadType) {
        // TODO: need to see how naga outputs MSL
        let offset = match vbo {
            QuadType::Offscreen => 0,
            QuadType::Final => 4,
        };

        unsafe {
            cmd.setVertexBuffer_offset_atIndex(Some(self.buffer.as_ref()), 0, 0);
            cmd.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveTypeTriangleStrip, offset, 4);
        }
    }
}
