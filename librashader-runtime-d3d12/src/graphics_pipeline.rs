use windows::core::Vtable;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Direct3D12::{D3D12_BLEND_DESC, D3D12_BLEND_INV_SRC_ALPHA, D3D12_BLEND_OP, D3D12_BLEND_OP_ADD, D3D12_BLEND_SRC_ALPHA, D3D12_COLOR_WRITE_ENABLE_ALL, D3D12_CULL_MODE_NONE, D3D12_DESCRIPTOR_RANGE, D3D12_DESCRIPTOR_RANGE_TYPE, D3D12_DESCRIPTOR_RANGE_TYPE_SAMPLER, D3D12_DESCRIPTOR_RANGE_TYPE_SRV, D3D12_FILL_MODE_SOLID, D3D12_GRAPHICS_PIPELINE_STATE_DESC, D3D12_INPUT_LAYOUT_DESC, D3D12_LOGIC_OP_NOOP, D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE, D3D12_RASTERIZER_DESC, D3D12_RENDER_TARGET_BLEND_DESC, D3D12_ROOT_DESCRIPTOR, D3D12_ROOT_DESCRIPTOR_TABLE, D3D12_ROOT_PARAMETER, D3D12_ROOT_PARAMETER_0, D3D12_ROOT_PARAMETER_TYPE_CBV, D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, D3D12_ROOT_SIGNATURE_DESC, D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT, D3D12_SHADER_BYTECODE, D3D12_SHADER_VISIBILITY_ALL, D3D12_SHADER_VISIBILITY_PIXEL, D3D12SerializeRootSignature, D3D_ROOT_SIGNATURE_VERSION_1, D3D_ROOT_SIGNATURE_VERSION_1_0, ID3D12Device, ID3D12PipelineState, ID3D12RootSignature};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC};
use librashader_reflect::back::cross::CrossHlslContext;
use librashader_reflect::back::ShaderCompilerOutput;
use crate::{error, util};
use crate::quad_render::DrawQuad;

pub struct D3D12GraphicsPipeline {
    pipeline_state: ID3D12PipelineState,
}

const D3D12_SLANG_ROOT_PARAMETERS: &'static [D3D12_ROOT_PARAMETER; 4] = &[
    // srvs
    D3D12_ROOT_PARAMETER {
        ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
        Anonymous: D3D12_ROOT_PARAMETER_0 {
            DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
                NumDescriptorRanges: 1,
                pDescriptorRanges: &D3D12_DESCRIPTOR_RANGE {
                    RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
                    NumDescriptors: 16,
                    BaseShaderRegister: 0,
                    RegisterSpace: 0,
                    OffsetInDescriptorsFromTableStart: 0,
                },
            }
        },
        ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
    },
    // samplers
    D3D12_ROOT_PARAMETER {
        ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
        Anonymous: D3D12_ROOT_PARAMETER_0 {
            DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
                NumDescriptorRanges: 1,
                pDescriptorRanges: &D3D12_DESCRIPTOR_RANGE {
                    RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SAMPLER,
                    NumDescriptors: 16,
                    BaseShaderRegister: 0,
                    RegisterSpace: 0,
                    OffsetInDescriptorsFromTableStart: 0,
                },
            }
        },
        ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
    },

    // UBO
    D3D12_ROOT_PARAMETER {
        ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
        Anonymous: D3D12_ROOT_PARAMETER_0 {
            Descriptor: D3D12_ROOT_DESCRIPTOR {
                ShaderRegister: 0,
                RegisterSpace: 0,
            }
        },
        ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
    },
    D3D12_ROOT_PARAMETER {
        ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
        Anonymous: D3D12_ROOT_PARAMETER_0 {
            Descriptor: D3D12_ROOT_DESCRIPTOR {
                ShaderRegister: 1,
                RegisterSpace: 0,
            }
        },
        ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
    }
];

const D3D12_SLANG_ROOT_SIGNATURE: &'static D3D12_ROOT_SIGNATURE_DESC = &D3D12_ROOT_SIGNATURE_DESC {
    NumParameters: D3D12_SLANG_ROOT_PARAMETERS.len() as u32,
    pParameters: D3D12_SLANG_ROOT_PARAMETERS.as_ptr(),
    NumStaticSamplers: 0,
    pStaticSamplers: std::ptr::null(),
    Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
};

pub struct D3D12RootSignature {
    signature: ID3D12RootSignature
}

impl D3D12RootSignature {
    pub fn new(device: &ID3D12Device)
        -> error::Result<D3D12RootSignature>
    {
        let signature = unsafe {
            let mut rs_blob = None;
            // todo: D3D12SerializeVersionedRootSignature
            // todo: hlsl rootsig tbh
            D3D12SerializeRootSignature(D3D12_SLANG_ROOT_SIGNATURE,
                                        D3D_ROOT_SIGNATURE_VERSION_1,
                                        &mut rs_blob,
                                        None
            )?;

            // SAFETY: if D3D12SerializeRootSignature succeeds then blob is Some
            let rs_blob = rs_blob.unwrap();
            let blob = std::slice::from_raw_parts(rs_blob.GetBufferPointer().cast(),
                                                  rs_blob.GetBufferSize());
            let root_signature: ID3D12RootSignature = device.CreateRootSignature(
                0, blob
            )?;
            root_signature
        };

        Ok(D3D12RootSignature {
            signature,
        })
    }
}
impl D3D12GraphicsPipeline {
    pub fn new(device: &ID3D12Device,
               shader_assembly: &ShaderCompilerOutput<String, CrossHlslContext>,
               root_signature: &D3D12RootSignature,
               render_format: DXGI_FORMAT
    ) -> error::Result<D3D12GraphicsPipeline> {
        let vertex_dxbc =
            util::d3d_compile_shader(shader_assembly.vertex.as_bytes(),
                                     b"main\0", b"vs_5_0\0")?;
        let fragment_dxbc =
            util::d3d_compile_shader(shader_assembly.fragment.as_bytes(), b"main\0",
                                     b"ps_5_0\0")?;

        let input_element = DrawQuad::get_spirv_cross_vbo_desc();


        let pipeline_state: ID3D12PipelineState = unsafe {
            let pipeline_desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                pRootSignature: windows::core::ManuallyDrop::new(&root_signature.signature),
                VS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: vertex_dxbc.GetBufferPointer(),
                    BytecodeLength: vertex_dxbc.GetBufferSize(),
                },
                PS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: fragment_dxbc.GetBufferPointer(),
                    BytecodeLength: fragment_dxbc.GetBufferSize(),
                },
                StreamOutput: Default::default(),
                BlendState: D3D12_BLEND_DESC {
                    RenderTarget: [
                        D3D12_RENDER_TARGET_BLEND_DESC {
                            BlendEnable: BOOL::from(false),
                            LogicOpEnable: BOOL::from(false),
                            SrcBlend: D3D12_BLEND_SRC_ALPHA,
                            DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
                            BlendOp: D3D12_BLEND_OP_ADD,
                            SrcBlendAlpha: D3D12_BLEND_SRC_ALPHA,
                            DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
                            BlendOpAlpha: D3D12_BLEND_OP_ADD,
                            LogicOp: D3D12_LOGIC_OP_NOOP,
                            RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
                        },
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default()
                    ],
                    ..Default::default()
                },
                SampleMask: u32::MAX,
                RasterizerState: D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_NONE,
                    ..Default::default()
                },
                DepthStencilState: Default::default(),
                InputLayout: D3D12_INPUT_LAYOUT_DESC {
                    pInputElementDescs: input_element.as_ptr(),
                    NumElements: input_element.len() as u32,
                },
                PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                NumRenderTargets: 1,
                RTVFormats: [
                    render_format,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_FORMAT_UNKNOWN,
                ],
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                NodeMask: 0,
                ..Default::default()
            };

            device.CreateGraphicsPipelineState(&pipeline_desc)?
        };

        Ok(D3D12GraphicsPipeline {
            pipeline_state,
        })
    }
}