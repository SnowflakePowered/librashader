use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D12::{D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER, D3D12_GPU_DESCRIPTOR_HANDLE, ID3D12DescriptorHeap, ID3D12Device};
use crate::error;

#[const_trait]
pub trait D3D12HeapType {
    fn get_desc(size: usize) -> D3D12_DESCRIPTOR_HEAP_DESC;
}
pub struct SamplerHeap;

impl const D3D12HeapType for SamplerHeap
{
    fn get_desc(size: usize) -> D3D12_DESCRIPTOR_HEAP_DESC {
        D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
            NumDescriptors: size as u32,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
            NodeMask: 0,
        }
    }
}

pub struct D3D12DescriptorHeapSlot<T> {
    cpu_handle: D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu_handle: D3D12_GPU_DESCRIPTOR_HANDLE,
    heap: Arc<RefCell<D3D12DescriptorHeapInner>>,
    slot: usize,
    _pd: PhantomData<T>
}

impl<T> D3D12DescriptorHeapSlot<T> {
    /// Get the index of the resource within the heap.
    pub fn index(&self) -> usize {
        self.slot
    }
}

impl<T> AsRef<D3D12_CPU_DESCRIPTOR_HANDLE> for D3D12DescriptorHeapSlot<T> {
    fn as_ref(&self) -> &D3D12_CPU_DESCRIPTOR_HANDLE {
        &self.cpu_handle
    }
}

impl<T> AsRef<D3D12_GPU_DESCRIPTOR_HANDLE> for D3D12DescriptorHeapSlot<T> {
    fn as_ref(&self) -> &D3D12_GPU_DESCRIPTOR_HANDLE {
        &self.gpu_handle
    }
}

struct D3D12DescriptorHeapInner {
    heap: ID3D12DescriptorHeap,
    desc: D3D12_DESCRIPTOR_HEAP_DESC,
    cpu_start: D3D12_CPU_DESCRIPTOR_HANDLE,
    gpu_start: D3D12_GPU_DESCRIPTOR_HANDLE,
    handle_size: usize,
    start: usize,
    // Bit flag representation of available handles in the heap.
    //
    //  0 - Occupied
    //  1 - free
    map: Box<[bool]>,
}

pub struct D3D12DescriptorHeap<T>(Arc<RefCell<D3D12DescriptorHeapInner>>, PhantomData<T>);

impl<T:D3D12HeapType> D3D12DescriptorHeap<T> {
    pub fn new(device: &ID3D12Device, size: usize) -> error::Result<D3D12DescriptorHeap<T>> {
        let desc = T::get_desc(size);
        unsafe {
            D3D12DescriptorHeap::new_with_desc(device, desc)
        }
    }
}

impl<T> D3D12DescriptorHeap<T> {
    pub unsafe fn new_with_desc(device: &ID3D12Device, desc: D3D12_DESCRIPTOR_HEAP_DESC) -> error::Result<D3D12DescriptorHeap<T>> {
        unsafe {
            let heap: ID3D12DescriptorHeap = device.CreateDescriptorHeap(&desc)?;
            let cpu_start = heap.GetCPUDescriptorHandleForHeapStart();
            let gpu_start =  heap.GetGPUDescriptorHandleForHeapStart();
            Ok(D3D12DescriptorHeap(Arc::new(RefCell::new(D3D12DescriptorHeapInner {
                heap,
                desc,
                cpu_start,
                gpu_start,
                handle_size: device.GetDescriptorHandleIncrementSize(desc.Type) as usize,
                start: 0,
                map: vec![false; desc.NumDescriptors as usize].into_boxed_slice(),
            })), PhantomData::default()))
        }
    }

    pub fn alloc_slot(&mut self) -> error::Result<D3D12DescriptorHeapSlot<T>> {
        let mut handle = D3D12_CPU_DESCRIPTOR_HANDLE { ptr: 0 };

        let mut inner = self.0.borrow_mut();
        for i in inner.start..inner.desc.NumDescriptors as usize {
            if !inner.map[i] {
                inner.map[i] = true;
                handle.ptr = inner.cpu_start.ptr + (i * inner.handle_size);
                inner.start = i + 1;

                let gpu_handle = D3D12_GPU_DESCRIPTOR_HANDLE {
                    ptr: (handle.ptr as u64 - inner.cpu_start.ptr as u64 + inner.gpu_start.ptr),
                };
                return Ok(D3D12DescriptorHeapSlot {
                    cpu_handle: handle,
                    slot: i,
                    heap: Arc::clone(&self.0),
                    gpu_handle,
                    _pd: Default::default(),
                });
            }
        }

        todo!("error need to fail");
    }
}

impl<T> Drop for D3D12DescriptorHeapSlot<T> {
    fn drop(&mut self) {
        let mut inner = self.heap.borrow_mut();
        inner.map[self.slot] = false;
        if inner.start > self.slot {
            inner.start = self.slot
        }
    }
}
