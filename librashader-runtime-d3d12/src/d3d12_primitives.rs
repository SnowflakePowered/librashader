use std::sync::Arc;
use windows::Win32::Graphics::Direct3D12::{D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_DESC, D3D12_GPU_DESCRIPTOR_HANDLE, ID3D12DescriptorHeap, ID3D12Device};
use crate::error;

pub struct D3D12DescriptorHeapSlot {
    cpu_handle: D3D12_CPU_DESCRIPTOR_HANDLE,
    heap: Arc<D3D12DescriptorHeap>
}

pub struct D3D12DescriptorHeap {
    heap: ID3D12DescriptorHeap,
    desc: D3D12_DESCRIPTOR_HEAP_DESC,
    cpu_handle: D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu_handle: D3D12_GPU_DESCRIPTOR_HANDLE,
    alignment: u32,
    map: Box<[bool]>,
    start: usize
}

impl D3D12DescriptorHeap {
    pub fn new(device: &ID3D12Device, desc: D3D12_DESCRIPTOR_HEAP_DESC) -> error::Result<Arc<D3D12DescriptorHeap>> {
        unsafe {
            let heap: ID3D12DescriptorHeap = device.CreateDescriptorHeap(&desc)?;
            let cpu_handle = heap.GetCPUDescriptorHandleForHeapStart();
            let gpu_handle = heap.GetGPUDescriptorHandleForHeapStart();
            let alignment = device.GetDescriptorHandleIncrementSize(desc.Type);
            let mut map = Vec::new();
            map.resize(desc.NumDescriptors as usize, false);

            Ok(Arc::new(D3D12DescriptorHeap {
                heap,
                desc,
                cpu_handle,
                gpu_handle,
                alignment,
                map: Box::new([]),
                start: 0,
            }))
        }
    }

    pub fn allocate_slot(self: &Arc<D3D12DescriptorHeap>) -> error::Result<D3D12DescriptorHeapSlot> {
        let mut handle = D3D12_CPU_DESCRIPTOR_HANDLE { ptr: 0 };

        for i in self.start..self.desc.NumDescriptors as usize {
            if !self.map[i] {
                self.map[i] = true;
                handle.ptr = self.cpu_handle.ptr + (i * self.alignment) as u64;
                self.start = i + 1;
                return Ok(D3D12DescriptorHeapSlot {
                    cpu_handle: handle,
                    heap: Arc::clone(self),
                });
            }
        }

        todo!("error need to fail");
    }

    pub fn free_slot(&mut self) -> error::Result<D3D12_CPU_DESCRIPTOR_HANDLE> {
        let mut handle = D3D12_CPU_DESCRIPTOR_HANDLE { ptr: 0 };

        for i in self.start..self.desc.NumDescriptors as usize {
            if !self.map[i] {
                self.map[i] = true;
                handle.ptr = self.cpu_handle.ptr + (i * self.alignment) as u64;
                self.start = i + 1;
                return Ok(handle);
            }
        }

        todo!("error need to fail");
    }

}

impl Drop