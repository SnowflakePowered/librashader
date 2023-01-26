use windows::Win32::Graphics::Direct3D12::{D3D12_GPU_DESCRIPTOR_HANDLE, ID3D12Device, ID3D12PipelineState, ID3D12RootSignature};
use librashader_common::Size;
use crate::error;

pub struct D3D12MipmapGen {
    device: ID3D12Device,
    root_signature: ID3D12RootSignature,
    pipeline: ID3D12PipelineState,
}

impl D3D12MipmapGen {
    pub fn new(device: &ID3D12Device) -> error::Result<D3D12MipmapGen> {
        todo!()
    }

    pub fn generate_mipmaps(miplevels: u16, size: Size<u32>, handle: D3D12_GPU_DESCRIPTOR_HANDLE) {

    }
}