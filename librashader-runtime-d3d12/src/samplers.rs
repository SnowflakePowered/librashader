use rustc_hash::FxHashMap;
use windows::Win32::Graphics::Direct3D12::D3D12_GPU_DESCRIPTOR_HANDLE;
use librashader_common::{FilterMode, WrapMode};

pub struct SamplerSet {
    samplers: FxHashMap<(WrapMode, FilterMode), D3D12_GPU_DESCRIPTOR_HANDLE>,
    heap: D3D12Descriptor_heap
}
