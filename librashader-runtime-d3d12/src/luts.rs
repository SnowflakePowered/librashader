use crate::descriptor_heap::CpuStagingHeap;
use crate::error;
use crate::filter_chain::FrameResiduals;
use crate::mipmap::MipmapGenContext;
use crate::texture::InputTexture;
use crate::util::{d3d12_get_closest_format, d3d12_resource_transition, d3d12_update_subresources};
use d3d12_descriptor_heap::D3D12DescriptorHeap;
use gpu_allocator::d3d12::{
    Allocator, Resource, ResourceCategory, ResourceCreateDesc, ResourceStateOrBarrierLayout,
    ResourceType,
};
use gpu_allocator::MemoryLocation;
use librashader_common::{FilterMode, ImageFormat, WrapMode};
use librashader_runtime::image::Image;
use librashader_runtime::scaling::MipmapSize;
use parking_lot::Mutex;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D12::{
    ID3D12Device, ID3D12GraphicsCommandList, D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
    D3D12_FEATURE_DATA_FORMAT_SUPPORT, D3D12_FORMAT_SUPPORT1_MIP,
    D3D12_FORMAT_SUPPORT1_SHADER_SAMPLE, D3D12_FORMAT_SUPPORT1_TEXTURE2D,
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT, D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_BUFFER,
    D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
    D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_GENERIC_READ,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_SHADER_RESOURCE_VIEW_DESC,
    D3D12_SHADER_RESOURCE_VIEW_DESC_0, D3D12_SRV_DIMENSION_TEXTURE2D, D3D12_SUBRESOURCE_DATA,
    D3D12_TEX2D_SRV, D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;

pub struct LutTexture {
    resource: ManuallyDrop<Resource>,
    view: InputTexture,
    miplevels: Option<u16>,
    // Staging heap needs to be kept alive until the command list is submitted, which is
    // really annoying. We could probably do better but it's safer to keep it around.
    staging: ManuallyDrop<Resource>,
    allocator: Arc<Mutex<Allocator>>,
}

impl LutTexture {
    pub(crate) fn new(
        device: &ID3D12Device,
        allocator: &Arc<Mutex<Allocator>>,
        heap: &mut D3D12DescriptorHeap<CpuStagingHeap>,
        cmd: &ID3D12GraphicsCommandList,
        source: &Image,
        filter: FilterMode,
        wrap_mode: WrapMode,
        mipmap: bool,
        gc: &mut FrameResiduals,
    ) -> error::Result<LutTexture> {
        let miplevels = source.size.calculate_miplevels() as u16;
        let mut desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: 0,
            Width: source.size.width as u64,
            Height: source.size.height,
            DepthOrArraySize: 1,
            MipLevels: if mipmap { miplevels } else { 1 },
            Format: ImageFormat::R8G8B8A8Unorm.into(),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: Default::default(),
            Flags: Default::default(),
        };

        let mut format_support = D3D12_FEATURE_DATA_FORMAT_SUPPORT {
            Format: desc.Format,
            Support1: D3D12_FORMAT_SUPPORT1_TEXTURE2D | D3D12_FORMAT_SUPPORT1_SHADER_SAMPLE,
            ..Default::default()
        };

        if mipmap {
            desc.Flags |= D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS;
            format_support.Support1 |= D3D12_FORMAT_SUPPORT1_MIP;
        }

        desc.Format = d3d12_get_closest_format(device, format_support);
        let descriptor = heap.allocate_descriptor()?;

        // create handles on GPU
        let resource = allocator.lock().create_resource(&ResourceCreateDesc {
            name: "lut alloc",
            memory_location: MemoryLocation::GpuOnly,
            resource_category: ResourceCategory::OtherTexture,
            resource_desc: &desc,
            castable_formats: &[],
            clear_value: None,
            initial_state_or_layout: ResourceStateOrBarrierLayout::ResourceState(
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            ),
            resource_type: &ResourceType::Placed,
        })?;

        unsafe {
            let srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                Format: desc.Format,
                ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
                Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D12_TEX2D_SRV {
                        MipLevels: u32::MAX,
                        ..Default::default()
                    },
                },
            };

            device.CreateShaderResourceView(
                resource.resource(),
                Some(&srv_desc),
                *descriptor.as_ref(),
            );
        }

        let mut buffer_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            ..Default::default()
        };

        let mut layout = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
        let mut total = 0;
        // texture upload
        unsafe {
            device.GetCopyableFootprints(
                &desc,
                0,
                1,
                0,
                Some(&mut layout),
                None,
                None,
                Some(&mut total),
            );

            buffer_desc.Width = total;
            buffer_desc.Height = 1;
            buffer_desc.DepthOrArraySize = 1;
            buffer_desc.MipLevels = 1;
            buffer_desc.SampleDesc.Count = 1;
            buffer_desc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
        }

        let upload = allocator.lock().create_resource(&ResourceCreateDesc {
            name: "lut staging",
            memory_location: MemoryLocation::CpuToGpu,
            resource_category: ResourceCategory::Buffer,
            resource_desc: &buffer_desc,
            castable_formats: &[],
            clear_value: None,
            initial_state_or_layout: ResourceStateOrBarrierLayout::ResourceState(
                D3D12_RESOURCE_STATE_GENERIC_READ,
            ),
            resource_type: &ResourceType::Placed,
        })?;

        let subresource = [D3D12_SUBRESOURCE_DATA {
            pData: source.bytes.as_ptr().cast(),
            RowPitch: 4 * source.size.width as isize,
            SlicePitch: (4 * source.size.width * source.size.height) as isize,
        }];

        gc.dispose_barriers(d3d12_resource_transition(
            cmd,
            &resource.resource(),
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
            D3D12_RESOURCE_STATE_COPY_DEST,
        ));

        d3d12_update_subresources(
            cmd,
            &resource.resource(),
            &upload.resource(),
            0,
            0,
            1,
            &subresource,
            gc,
        )?;

        gc.dispose_barriers(d3d12_resource_transition(
            cmd,
            &resource.resource(),
            D3D12_RESOURCE_STATE_COPY_DEST,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
        ));

        let view = InputTexture::new(
            resource.resource().clone(),
            descriptor,
            source.size,
            ImageFormat::R8G8B8A8Unorm.into(),
            filter,
            wrap_mode,
        );
        Ok(LutTexture {
            resource: ManuallyDrop::new(resource),
            staging: ManuallyDrop::new(upload),
            view,
            miplevels: if mipmap { Some(miplevels) } else { None },
            allocator: Arc::clone(&allocator),
        })
    }

    pub fn generate_mipmaps(&self, gen_mips: &mut MipmapGenContext) -> error::Result<()> {
        if let Some(miplevels) = self.miplevels {
            gen_mips.generate_mipmaps(
                &self.resource.resource(),
                miplevels,
                self.view.size,
                ImageFormat::R8G8B8A8Unorm.into(),
            )?
        }

        Ok(())
    }
}

impl AsRef<InputTexture> for LutTexture {
    fn as_ref(&self) -> &InputTexture {
        &self.view
    }
}

impl Drop for LutTexture {
    fn drop(&mut self) {
        let resource = unsafe { ManuallyDrop::take(&mut self.resource) };
        if let Err(e) = self.allocator.lock().free_resource(resource) {
            println!("librashader-runtime-d3d12: [warn] failed to deallocate lut buffer memory {e}")
        }

        let staging = unsafe { ManuallyDrop::take(&mut self.staging) };
        if let Err(e) = self.allocator.lock().free_resource(staging) {
            println!("librashader-runtime-d3d12: [warn] failed to deallocate lut staging buffer memory {e}")
        }
    }
}
