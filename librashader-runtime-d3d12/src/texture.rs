use windows::Win32::Graphics::Direct3D12::{D3D12_CPU_DESCRIPTOR_HANDLE};
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use crate::heap::{CpuStagingHeap, D3D12DescriptorHeapSlot};

enum InputDescriptor {
    Owned(D3D12DescriptorHeapSlot<CpuStagingHeap>),
    Raw(D3D12_CPU_DESCRIPTOR_HANDLE)
}

enum OutputDescriptor {
    Owned(D3D12DescriptorHeapSlot<CpuStagingHeap>),
    Raw(D3D12_CPU_DESCRIPTOR_HANDLE)
}

impl AsRef<D3D12_CPU_DESCRIPTOR_HANDLE> for InputDescriptor {
    fn as_ref(&self) -> &D3D12_CPU_DESCRIPTOR_HANDLE {
        match self {
            InputDescriptor::Owned(h) => h.as_ref(),
            InputDescriptor::Raw(h) => h
        }
    }
}

impl AsRef<D3D12_CPU_DESCRIPTOR_HANDLE> for OutputDescriptor {
    fn as_ref(&self) -> &D3D12_CPU_DESCRIPTOR_HANDLE {
        match self {
            OutputDescriptor::Owned(h) => h.as_ref(),
            OutputDescriptor::Raw(h) => h
        }
    }
}

pub struct OutputTexture {
    descriptor: OutputDescriptor,
    size: Size<u32>,
}

impl OutputTexture {
    pub fn new(handle: D3D12DescriptorHeapSlot<CpuStagingHeap>,
               size: Size<u32>,
              ) -> OutputTexture {
        let descriptor = OutputDescriptor::Owned(handle);
        OutputTexture {
            descriptor,
            size,
        }
    }

    // unsafe since the lifetime of the handle has to survive
    pub unsafe fn new_from_raw(handle: D3D12_CPU_DESCRIPTOR_HANDLE,
                               size: Size<u32>,
    ) -> OutputTexture {
        let descriptor = OutputDescriptor::Raw(handle);
        OutputTexture {
            descriptor,
            size,
        }
    }
}

pub struct InputTexture {
    descriptor: InputDescriptor,
    pub(crate) size: Size<u32>,
    format: ImageFormat,
    wrap_mode: WrapMode,
    filter: FilterMode
}

impl InputTexture {
    pub fn new(handle: D3D12DescriptorHeapSlot<CpuStagingHeap>,
               size: Size<u32>,
               format: ImageFormat,
               wrap_mode: WrapMode,
               filter: FilterMode) -> InputTexture {
        let srv = InputDescriptor::Owned(handle);
        InputTexture {
            descriptor: srv,
            size,
            format,
            wrap_mode,
            filter
        }
    }

    // unsafe since the lifetime of the handle has to survive
    pub unsafe fn new_from_raw(handle: D3D12_CPU_DESCRIPTOR_HANDLE,
                               size: Size<u32>,
                               format: ImageFormat,
                               wrap_mode: WrapMode,
                               filter: FilterMode
    ) -> InputTexture {
        let srv = InputDescriptor::Raw(handle);
        InputTexture {
            descriptor: srv,
            size,
            format,
            wrap_mode,
            filter
        }
    }

}
