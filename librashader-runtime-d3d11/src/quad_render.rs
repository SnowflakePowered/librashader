use crate::error;
use crate::error::assume_d3d11_init;
use bytemuck::offset_of;
use windows::core::PCSTR;
use windows::Win32::Graphics::Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC,
    D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_SUBRESOURCE_DATA,
    D3D11_USAGE_IMMUTABLE,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32G32_FLOAT;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
struct D3D11Vertex {
    position: [f32; 2],
    texcoord: [f32; 2],
    color: [f32; 4],
}

const CLEAR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

static QUAD_VBO_DATA: &[D3D11Vertex; 4] = &[
    D3D11Vertex {
        position: [0.0, 0.0],
        texcoord: [0.0, 1.0],
        color: CLEAR,
    },
    D3D11Vertex {
        position: [0.0, 1.0],
        texcoord: [0.0, 0.0],
        color: CLEAR,
    },
    D3D11Vertex {
        position: [1.0, 0.0],
        texcoord: [1.0, 1.0],
        color: CLEAR,
    },
    D3D11Vertex {
        position: [1.0, 1.0],
        texcoord: [1.0, 0.0],
        color: CLEAR,
    },
];

pub(crate) struct DrawQuad {
    buffer: ID3D11Buffer,
    context: ID3D11DeviceContext,
    offset: u32,
    stride: u32,
}

impl DrawQuad {
    pub fn new(device: &ID3D11Device, context: &ID3D11DeviceContext) -> error::Result<DrawQuad> {
        unsafe {
            let mut buffer = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: std::mem::size_of::<[D3D11Vertex; 4]>() as u32,
                    Usage: D3D11_USAGE_IMMUTABLE,
                    BindFlags: D3D11_BIND_VERTEX_BUFFER,
                    CPUAccessFlags: Default::default(),
                    MiscFlags: Default::default(),
                    StructureByteStride: 0,
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: QUAD_VBO_DATA.as_ptr().cast(),
                    SysMemPitch: 0,
                    SysMemSlicePitch: 0,
                }),
                Some(&mut buffer),
            )?;
            assume_d3d11_init!(buffer, "CreateBuffer");

            Ok(DrawQuad {
                buffer,
                context: context.clone(),
                offset: 0,
                stride: std::mem::size_of::<D3D11Vertex>() as u32,
            })
        }
    }

    pub fn bind_vertices(&self) {
        unsafe {
            self.context
                .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            self.context.IASetVertexBuffers(
                0,
                1,
                Some(&Some(self.buffer.clone())),
                Some(&self.stride),
                Some(&self.offset),
            );
        }
    }

    pub fn get_spirv_cross_vbo_desc() -> [D3D11_INPUT_ELEMENT_DESC; 2] {
        [
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(b"TEXCOORD\0".as_ptr()),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: offset_of!(D3D11Vertex, position) as u32,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(b"TEXCOORD\0".as_ptr()),
                SemanticIndex: 1,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: offset_of!(D3D11Vertex, texcoord) as u32,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ]
    }
}
