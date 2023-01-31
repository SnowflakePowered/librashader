use crate::error;
use bytemuck::{offset_of, Pod, Zeroable};
use windows::core::PCSTR;
use windows::w;
use windows::Win32::Graphics::Direct3D::{D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP};
use windows::Win32::Graphics::Direct3D12::{D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, D3D12_INPUT_ELEMENT_DESC, D3D12_VERTEX_BUFFER_VIEW, ID3D12CommandList, ID3D12Device, ID3D12GraphicsCommandList, ID3D12Resource};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32G32_FLOAT;
use crate::buffer::{D3D12Buffer, D3D12ConstantBuffer};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Zeroable, Pod)]
struct D3D12Vertex {
    position: [f32; 2],
    texcoord: [f32; 2],
    color: [f32; 4],
}

const CLEAR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

static QUAD_VBO_DATA: &[D3D12Vertex; 4] = &[
    D3D12Vertex {
        position: [0.0, 0.0],
        texcoord: [0.0, 1.0],
        color: CLEAR,
    },
    D3D12Vertex {
        position: [0.0, 1.0],
        texcoord: [0.0, 0.0],
        color: CLEAR,
    },
    D3D12Vertex {
        position: [1.0, 0.0],
        texcoord: [1.0, 1.0],
        color: CLEAR,
    },
    D3D12Vertex {
        position: [1.0, 1.0],
        texcoord: [1.0, 0.0],
        color: CLEAR,
    },
];

pub(crate) struct DrawQuad {
    buffer: ID3D12Resource,
    view: D3D12_VERTEX_BUFFER_VIEW,
}

impl DrawQuad {
    pub fn new(device: &ID3D12Device)
        -> error::Result<DrawQuad> {
        let stride = std::mem::size_of::<D3D12Vertex>() as u32;
        let size = std::mem::size_of::<[D3D12Vertex;4]>() as u32;
        let mut buffer = D3D12Buffer::new(device, size as usize)?;
        buffer.map(None)?
            .slice.copy_from_slice(bytemuck::cast_slice(QUAD_VBO_DATA));

        let view = D3D12_VERTEX_BUFFER_VIEW {
            BufferLocation: buffer.gpu_address(),
            SizeInBytes: size,
            StrideInBytes: stride,
        };

        let buffer = buffer.into_raw();
        unsafe  {
            buffer.SetName(w!("drawquad"))?;
        }
        Ok(DrawQuad {
            buffer,
            view
        })
    }

    pub fn bind_vertices(&self, cmd: &ID3D12GraphicsCommandList) {
        unsafe {
            cmd.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            cmd.IASetVertexBuffers(0, Some(&[self.view]));
        }
    }

    pub fn get_spirv_cross_vbo_desc() -> [D3D12_INPUT_ELEMENT_DESC; 2] {
        [
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(b"TEXCOORD\0".as_ptr()),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: offset_of!(D3D12Vertex, position) as u32,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(b"TEXCOORD\0".as_ptr()),
                SemanticIndex: 1,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: offset_of!(D3D12Vertex, texcoord) as u32,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ]
    }
}
