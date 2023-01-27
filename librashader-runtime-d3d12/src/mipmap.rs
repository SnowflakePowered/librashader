use windows::Win32::Graphics::Direct3D12::{D3D12_COMPUTE_PIPELINE_STATE_DESC, D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_RANGE, D3D12_DESCRIPTOR_RANGE_TYPE_SRV, D3D12_DESCRIPTOR_RANGE_TYPE_UAV, D3D12_FILTER_MIN_MAG_LINEAR_MIP_POINT, D3D12_GPU_DESCRIPTOR_HANDLE, D3D12_ROOT_CONSTANTS, D3D12_ROOT_DESCRIPTOR_TABLE, D3D12_ROOT_PARAMETER, D3D12_ROOT_PARAMETER_0, D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS, D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, D3D12_ROOT_SIGNATURE_DESC, D3D12_ROOT_SIGNATURE_FLAG_DENY_DOMAIN_SHADER_ROOT_ACCESS, D3D12_ROOT_SIGNATURE_FLAG_DENY_GEOMETRY_SHADER_ROOT_ACCESS, D3D12_ROOT_SIGNATURE_FLAG_DENY_HULL_SHADER_ROOT_ACCESS, D3D12_ROOT_SIGNATURE_FLAG_DENY_PIXEL_SHADER_ROOT_ACCESS, D3D12_ROOT_SIGNATURE_FLAG_DENY_VERTEX_SHADER_ROOT_ACCESS, D3D12_SHADER_BYTECODE, D3D12_SHADER_VISIBILITY_ALL, D3D12_STATIC_SAMPLER_DESC, D3D12_TEXTURE_ADDRESS_MODE_CLAMP, ID3D12Device, ID3D12PipelineState, ID3D12RootSignature};
use librashader_common::Size;
use crate::error;
use crate::util::d3d_compile_shader;

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
\"RootConstants(num32BitConstants=3, b0),\" \\
\"DescriptorTable ( SRV(t0) ),\" \\
\"DescriptorTable ( UAV(u0) ),\" \\
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

impl D3D12MipmapGen {
    pub fn new(device: &ID3D12Device) -> error::Result<D3D12MipmapGen> {
        unsafe {
            let blob = d3d_compile_shader(GENERATE_MIPS_SRC, b"main\0", b"cs_5_1\0")?;
            let blob = std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize());
            let root_signature: ID3D12RootSignature = device.CreateRootSignature(0, blob)?;

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

    pub fn generate_mipmaps(miplevels: u16,
                            size: Size<u32>,
                            handle: D3D12_CPU_DESCRIPTOR_HANDLE) {

    }
}