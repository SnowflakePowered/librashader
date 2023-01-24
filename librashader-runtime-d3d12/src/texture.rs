use windows::Win32::Graphics::Direct3D12::{D3D12_RESOURCE_DESC, ID3D12Device, ID3D12Resource};
use librashader_common::{FilterMode, WrapMode};
use librashader_runtime::image::Image;

pub struct LutTexture {
    handle: ID3D12Resource,

}

impl LutTexture {
    pub fn new(
        device: &ID3D12Device,
        source: &Image,
        desc: D3D12_RESOURCE_DESC,
        filter: FilterMode,
        wrap_mode: WrapMode,
    ) {
        // todo: d3d12:800
    }
}