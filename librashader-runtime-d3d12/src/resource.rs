use crate::filter_chain::FrameResiduals;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use windows::Win32::Graphics::Direct3D12::ID3D12Resource;

pub trait ResourceHandleStrategy<H> {
    unsafe fn obtain(handle: &H) -> ManuallyDrop<Option<ID3D12Resource>>;
    unsafe fn cleanup(gc: &mut FrameResiduals, handle: ManuallyDrop<Option<ID3D12Resource>>);
}

pub struct IncrementRefcount;

impl ResourceHandleStrategy<ManuallyDrop<ID3D12Resource>> for IncrementRefcount {
    unsafe fn obtain(
        handle: &ManuallyDrop<ID3D12Resource>,
    ) -> ManuallyDrop<Option<ID3D12Resource>> {
        ManuallyDrop::new(Some(handle.deref().clone()))
    }

    unsafe fn cleanup(gc: &mut FrameResiduals, handle: ManuallyDrop<Option<ID3D12Resource>>) {
        gc.dispose_resource(handle);
    }
}

pub struct OutlivesFrame;

impl ResourceHandleStrategy<ManuallyDrop<ID3D12Resource>> for OutlivesFrame {
    unsafe fn obtain(
        handle: &ManuallyDrop<ID3D12Resource>,
    ) -> ManuallyDrop<Option<ID3D12Resource>> {
        unsafe { std::mem::transmute_copy(handle) }
    }

    unsafe fn cleanup(_gc: &mut FrameResiduals, _handle: ManuallyDrop<Option<ID3D12Resource>>) {
        // Since the lifetime is ensured for the lifetime of the filter chain strategy, do nothing
    }
}
