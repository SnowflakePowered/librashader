
use std::mem::ManuallyDrop;
use windows::Win32::Graphics::Direct3D12::{D3D12_COMPUTE_PIPELINE_STATE_DESC, D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING, D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_TYPE_UAV, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_UAV_BARRIER, D3D12_SHADER_BYTECODE, D3D12_SHADER_RESOURCE_VIEW_DESC, D3D12_SHADER_RESOURCE_VIEW_DESC_0, D3D12_SRV_DIMENSION_TEXTURE2D, D3D12_TEX2D_SRV, D3D12_TEX2D_UAV, D3D12_UAV_DIMENSION_TEXTURE2D, D3D12_UNORDERED_ACCESS_VIEW_DESC, D3D12_UNORDERED_ACCESS_VIEW_DESC_0, ID3D12DescriptorHeap, ID3D12Device, ID3D12GraphicsCommandList, ID3D12PipelineState, ID3D12Resource, ID3D12RootSignature};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;
use librashader_common::Size;
use crate::{error, util};
use crate::heap::{D3D12DescriptorHeap, D3D12DescriptorHeapSlot, ResourceWorkHeap};
use crate::util::d3d_compile_shader;
use bytemuck::{Zeroable, Pod};
use librashader_runtime::scaling::MipmapSize;

static GENERATE_MIPS_SRC: &[u8] = b"
// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.
//
// http://go.microsoft.com/fwlink/?LinkID=615561

#define GenerateMipsRS \\
\"RootFlags ( DENY_VERTEX_SHADER_ROOT_ACCESS   |\" \\
\"            DENY_DOMAIN_SHADER_ROOT_ACCESS   |\" \\
\"            DENY_GEOMETRY_SHADER_ROOT_ACCESS |\" \\
\"            DENY_HULL_SHADER_ROOT_ACCESS     |\" \\
\"            DENY_PIXEL_SHADER_ROOT_ACCESS ),\" \\
\"DescriptorTable ( SRV(t0, flags=DATA_VOLATILE) ),\" \\
\"DescriptorTable ( UAV(u0, flags=DATA_VOLATILE) ),\" \\
\"RootConstants(num32BitConstants=3, b0),\" \\
\"StaticSampler(s0,\"\\
\"           filter =   FILTER_MIN_MAG_LINEAR_MIP_POINT,\"\\
\"           addressU = TEXTURE_ADDRESS_CLAMP,\"\\
\"           addressV = TEXTURE_ADDRESS_CLAMP,\"\\
\"           addressW = TEXTURE_ADDRESS_CLAMP )\"

SamplerState Sampler       : register(s0);
Texture2D<float4> SrcMip   : register(t0);
RWTexture2D<float4> OutMip : register(u0);

cbuffer MipConstants : register(b0)
{
float2 InvOutTexelSize; // texel size for OutMip (NOT SrcMip)
uint SrcMipIndex;
}

float4 Mip(uint2 coord)
{
    float2 uv = (coord.xy + 0.5) * InvOutTexelSize;
    return SrcMip.SampleLevel(Sampler, uv, SrcMipIndex);
}

[RootSignature(GenerateMipsRS)]
[numthreads(8, 8, 1)]
void main(uint3 DTid : SV_DispatchThreadID)
{
OutMip[DTid.xy] = Mip(DTid.xy);
}\0";

pub struct D3D12MipmapGen {
    device: ID3D12Device,
    root_signature: ID3D12RootSignature,
    pipeline: ID3D12PipelineState,
}

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct MipConstants {
    inv_out_texel_size: [f32; 2],
    src_mip_index: u32,
}

pub struct MipmapGenContext<'a> {
    gen: &'a D3D12MipmapGen,
    cmd: &'a ID3D12GraphicsCommandList,
    heap: &'a mut D3D12DescriptorHeap<ResourceWorkHeap>,
    residuals: Vec<D3D12DescriptorHeapSlot<ResourceWorkHeap>>
}

impl <'a> MipmapGenContext<'a> {
    fn new(gen: &'a D3D12MipmapGen, cmd: &'a ID3D12GraphicsCommandList, heap: &'a mut D3D12DescriptorHeap<ResourceWorkHeap>) -> MipmapGenContext<'a> {
        Self {
            gen, cmd, heap, residuals: Vec::new()
        }
    }

    /// Generate a set of mipmaps for the resource.
    /// This is a "cheap" action and only dispatches a compute shader.
    pub fn generate_mipmaps(&mut self, resource: &ID3D12Resource, miplevels: u16, size: Size<u32>, format: DXGI_FORMAT)
    -> error::Result<()>
    {
        unsafe {
            let residuals = self.gen.generate_mipmaps(self.cmd, resource, miplevels, size, format, self.heap)?;
            self.residuals.extend(residuals)
        }

        Ok(())
    }

    fn close(self) -> Vec<D3D12DescriptorHeapSlot<ResourceWorkHeap>> {
        self.residuals
    }
}

impl D3D12MipmapGen {
    pub fn new(device: &ID3D12Device) -> error::Result<D3D12MipmapGen> {
        unsafe {
            let blob = d3d_compile_shader(GENERATE_MIPS_SRC, b"main\0", b"cs_5_1\0").unwrap();
            let blob = std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize());
            let root_signature: ID3D12RootSignature = device.CreateRootSignature(0, blob).unwrap();

            let desc = D3D12_COMPUTE_PIPELINE_STATE_DESC {
                pRootSignature: windows::core::ManuallyDrop::new(&root_signature),
                CS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: blob.as_ptr().cast(),
                    BytecodeLength: blob.len()
                },
                NodeMask: 0,
                ..Default::default()
            };

            let pipeline = device.CreateComputePipelineState(&desc)?;

            Ok(D3D12MipmapGen {
                device: device.clone(),
                root_signature,
                pipeline,
            })
        }
    }

    /// Enters a mipmapping compute context.
    /// This is a relatively expensive operation
    /// and should only be done at most a few times per frame.
    pub fn mipmapping_context<F>(&self, cmd: &ID3D12GraphicsCommandList, work_heap: &mut D3D12DescriptorHeap<ResourceWorkHeap>, mut f: F)
    -> error::Result<Vec<D3D12DescriptorHeapSlot<ResourceWorkHeap>>>
        where
            F: FnMut(&mut MipmapGenContext)
    {
        let heap: ID3D12DescriptorHeap = (&(*work_heap)).into();
        unsafe {
            cmd.SetComputeRootSignature(&self.root_signature);
            cmd.SetPipelineState(&self.pipeline);
            cmd.SetDescriptorHeaps(&[heap]);
        }

        let mut context = MipmapGenContext::new(self, cmd, work_heap);
        f(&mut context);
        Ok(context.close())
    }

    /// SAFETY:
    ///   - handle must be a CPU handle to an SRV
    ///   - work_heap must have enough descriptors to fit all miplevels.
    unsafe fn generate_mipmaps(&self,
                                   cmd: &ID3D12GraphicsCommandList,
                                   resource: &ID3D12Resource,

                                   miplevels: u16,
                            size: Size<u32>,
                            format: DXGI_FORMAT,
                            work_heap: &mut D3D12DescriptorHeap<ResourceWorkHeap>) -> error::Result<Vec<D3D12DescriptorHeapSlot<ResourceWorkHeap>>>
    {
        // create views for mipmap generation
        let srv = work_heap.alloc_slot()?;
        {
            let srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
                Format: format,
                ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
                Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
                Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D12_TEX2D_SRV {
                        MipLevels: miplevels as u32,
                        ..Default::default()
                    },
                },
            };

            self.device.CreateShaderResourceView(resource,
                                                 Some(&srv_desc), *srv.as_ref());
        }

        let mut heap_slots = Vec::with_capacity(miplevels as usize);
        heap_slots.push(srv);

        for i in 1..miplevels {
            let descriptor = work_heap.alloc_slot()?;
            let desc = D3D12_UNORDERED_ACCESS_VIEW_DESC {
                Format: format,
                ViewDimension: D3D12_UAV_DIMENSION_TEXTURE2D,
                Anonymous: D3D12_UNORDERED_ACCESS_VIEW_DESC_0 {
                    Texture2D: D3D12_TEX2D_UAV {
                        MipSlice: i as u32,
                        ..Default::default()
                    }
                },
            };

            self.device.CreateUnorderedAccessView(resource, None,
                                                  Some(&desc), *descriptor.as_ref()
            );
            heap_slots.push(descriptor);
        }

        cmd.SetComputeRootDescriptorTable(0, *heap_slots[0].as_ref());

        for i in 1..miplevels as u32 {
            let scaled = size.scale_mipmap(i);
            let mipmap_params = MipConstants {
                inv_out_texel_size: [
                    1.0 / scaled.width as f32,
                    1.0 / scaled.height as f32
                ],
                src_mip_index: (i - 1),
            };

            let mipmap_params = bytemuck::bytes_of(&mipmap_params);


            util::d3d12_resource_transition_subresource(cmd, resource,
                                                        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                                                        D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
                                                        i - 1
            );

            util::d3d12_resource_transition_subresource(cmd, resource,
                                                        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                                                        D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
                i
            );

            cmd.SetComputeRootDescriptorTable(1, *heap_slots[i as usize].as_ref());
            cmd.SetComputeRoot32BitConstants(2,
                                             (std::mem::size_of::<MipConstants>() / std::mem::size_of::<u32>()) as u32,
                                             mipmap_params.as_ptr().cast(),
                                             0);

            cmd.Dispatch( std::cmp::max(scaled.width / 8, 1), std::cmp::max(scaled.height / 8, 1), 1);

            // todo: handle manuallyDrop properly.

            let uav_barrier = ManuallyDrop::new(D3D12_RESOURCE_UAV_BARRIER {
                pResource: windows::core::ManuallyDrop::new(resource),
            });

            let barrier = [D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_UAV,
                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    UAV: uav_barrier
                },
                ..Default::default()
            }];

            cmd.ResourceBarrier(&barrier);

            util::d3d12_resource_transition_subresource(cmd, resource,
                                                        D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
                                                        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                                                        i
            );
        }

        Ok(heap_slots)
    }
}