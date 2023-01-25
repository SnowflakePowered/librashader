use crate::error;
use crate::error::assume_d3d12_init;
use crate::heap::{D3D12DescriptorHeap, D3D12DescriptorHeapSlot, LutTextureHeap};
use crate::util::d3d12_get_closest_format;
use librashader_common::{FilterMode, ImageFormat, Size, WrapMode};
use librashader_runtime::image::Image;
use windows::Win32::Graphics::Direct3D12::{ID3D12CommandList, ID3D12Device, ID3D12GraphicsCommandList, ID3D12Resource, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING, D3D12_FEATURE_DATA_FORMAT_SUPPORT, D3D12_FORMAT_SUPPORT1_SHADER_SAMPLE, D3D12_FORMAT_SUPPORT1_TEXTURE2D, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_DEFAULT, D3D12_HEAP_TYPE_UPLOAD, D3D12_MEMORY_POOL_UNKNOWN, D3D12_PLACED_SUBRESOURCE_FOOTPRINT, D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_STATE_GENERIC_READ, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_SHADER_RESOURCE_VIEW_DESC, D3D12_SHADER_RESOURCE_VIEW_DESC_0, D3D12_SRV_DIMENSION_TEXTURE2D, D3D12_TEX2D_SRV, D3D12_TEXTURE_LAYOUT_ROW_MAJOR, D3D12_RANGE};
use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;

pub struct LutTexture {
    resource: ID3D12Resource,
    descriptor: D3D12DescriptorHeapSlot<LutTextureHeap>,
    size: Size<u32>,
    filter: FilterMode,
    wrap_mode: WrapMode,
}

impl LutTexture {
    pub fn new(
        device: &ID3D12Device,
        heap: &mut D3D12DescriptorHeap<LutTextureHeap>,
        cmd: &ID3D12GraphicsCommandList,
        source: &Image,
        filter: FilterMode,
        wrap_mode: WrapMode,
        mipmap: bool,
    ) -> error::Result<LutTexture> {
        // todo: d3d12:800
        let mut desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: 0,
            Width: source.size.width as u64,
            Height: source.size.height,
            DepthOrArraySize: 1,
            MipLevels: 1, // todo: mipmaps
            // MipLevels: if mipmap { u16::MAX } else { 1 },
            Format: ImageFormat::R8G8B8A8Unorm.into(),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: Default::default(),
            Flags: Default::default(),
        };

        let format_support = D3D12_FEATURE_DATA_FORMAT_SUPPORT {
            Format: desc.Format,
            Support1: D3D12_FORMAT_SUPPORT1_TEXTURE2D | D3D12_FORMAT_SUPPORT1_SHADER_SAMPLE,
            ..Default::default()
        };

        desc.Format = d3d12_get_closest_format(device, desc.Format, format_support);
        let descriptor = heap.alloc_slot()?;

        // create handles on GPU
        let mut resource: Option<ID3D12Resource> = None;
        unsafe {
            device.CreateCommittedResource(
                &D3D12_HEAP_PROPERTIES {
                    Type: D3D12_HEAP_TYPE_DEFAULT,
                    CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                    MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                    CreationNodeMask: 1,
                    VisibleNodeMask: 1,
                },
                D3D12_HEAP_FLAG_NONE,
                &desc,
                D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                None,
                &mut resource,
            )?;
        }
        assume_d3d12_init!(resource, "CreateCommittedResource");
        unsafe {
            let srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                Format: desc.Format,
                ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
                Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D12_TEX2D_SRV {
                        MipLevels: desc.MipLevels as u32,
                        ..Default::default()
                    },
                },
            };

            device.CreateShaderResourceView(&resource, Some(&srv_desc), *descriptor.as_ref());
        }

        let mut buffer_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            ..Default::default()
        };

        let mut layout = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
        let mut numrows = 0;
        let mut rowsize = 0;
        let mut total = 0;
        // texture upload
        unsafe {
            device.GetCopyableFootprints(
                &desc,
                0,
                1,
                0,
                Some(&mut layout),
                Some(&mut numrows),
                Some(&mut rowsize),
                Some(&mut total),
            );

            buffer_desc.Width = total;
            buffer_desc.Height = 1;
            buffer_desc.DepthOrArraySize = 1;
            buffer_desc.MipLevels = 1;
            buffer_desc.SampleDesc.Count = 1;
            buffer_desc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
        }
        let mut upload: Option<ID3D12Resource> = None;

        unsafe {
            device.CreateCommittedResource(
                &D3D12_HEAP_PROPERTIES {
                    Type: D3D12_HEAP_TYPE_UPLOAD,
                    CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                    MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                    CreationNodeMask: 1,
                    VisibleNodeMask: 1,
                },
                D3D12_HEAP_FLAG_NONE,
                &buffer_desc,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &mut upload,
            )?;
        }
        assume_d3d12_init!(upload, "CreateCommittedResource");

        unsafe {
            let mut range = D3D12_RANGE {
                Begin: 0,
                End: 0
            };

            let mut buf = std::ptr::null_mut();

            upload.Map(0, Some(&range), Some(&mut buf))?;

            upload.Unmap(0, None);

        }
        // todo: upload image data to textur

        Ok(LutTexture {
            resource,
            descriptor,
            size: source.size,
            filter,
            wrap_mode,
        })
    }
}


// todo https://github.com/microsoft/DirectX-Graphics-Samples/blob/master/Libraries/D3D12RaytracingFallback/Include/d3dx12.h#L1893