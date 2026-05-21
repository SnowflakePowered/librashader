use crate::back::hlsl::{CrossHlslContext, HlslBufferAssignment, HlslBufferAssignments};
use crate::back::targets::HLSL;
use crate::back::{CompileShader, ShaderCompilerOutput};
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::front::spirv_passes::harden_normalize::HardenNormalize;
use crate::front::spirv_passes::load_module;
use crate::front::spirv_passes::lower_loop_sample_lod::LowerLoopSampleLod;
use crate::front::SpirvCompilation;
use crate::reflect::cross::{CompiledProgram, CrossReflect};
use crate::reflect::semantics::{ShaderReflection, ShaderSemantics};
use crate::reflect::ReflectShader;
use rspirv::binary::Assemble;
use rspirv::dr::Builder;
use spirv::Decoration;

use spirv_cross2::compile::hlsl::HlslShaderModel;
use spirv_cross2::compile::CompilableTarget;
use spirv_cross2::reflect::{DecorationValue, ResourceType};
use spirv_cross2::{targets, SpirvCrossError};

pub(crate) type HlslReflect = CrossReflect<targets::Hlsl>;

/// Wraps `HlslReflect` so we can defer the choice of shader model until
/// `compile()` time and re-build the spirv-cross compiler from a hardened
/// SPIR-V module when SM3 is targeted.
///
/// FXC SM3 rejects NaN/Inf constants that its own folder can synthesize from
/// idioms like `normalize((0,0))`. The fix lives in
/// `spirv_passes::harden_normalize`; we only pay the cost when actually
/// targeting SM3, keeping the cached SPIR-V (and the reflection used by
/// SM5+ targets) untouched.
pub(crate) struct HlslCompileShader {
    backend: HlslReflect,
    spirv: SpirvCompilation,
}

impl HlslCompileShader {
    pub(crate) fn new(spirv: SpirvCompilation) -> Result<Self, ShaderReflectError> {
        let backend = HlslReflect::try_from(&spirv)?;
        Ok(Self { backend, spirv })
    }
}

impl ReflectShader for HlslCompileShader {
    fn reflect(
        &mut self,
        pass_number: usize,
        semantics: &ShaderSemantics,
    ) -> Result<ShaderReflection, ShaderReflectError> {
        // The hardening pass only adds an OpFAdd before each Normalize; it
        // doesn't add/remove resources, interfaces, or semantics. Reflecting
        // against the un-hardened module yields the same result.
        self.backend.reflect(pass_number, semantics)
    }

    fn validate(&mut self) -> Result<(), ShaderReflectError> {
        self.backend.validate()
    }
}

impl CompileShader<HLSL> for HlslCompileShader {
    type Options = Option<HlslShaderModel>;
    type Context = CrossHlslContext;

    fn compile(
        self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, CrossHlslContext>, ShaderCompileError> {
        // Do some unholy patching to get some shaders to work on certain DirectX shader models.

        let sm = options.unwrap_or(HlslShaderModel::ShaderModel5_0);

        let lowering_passes = match sm {
            HlslShaderModel::ShaderModel3_0 => {
                static SM3_0_LOWER: fn(&mut Builder) = |builder: &mut Builder| {
                    HardenNormalize::new(builder).do_pass();
                    LowerLoopSampleLod::new(builder).do_pass();
                };
                Some(SM3_0_LOWER)
            }
            HlslShaderModel::ShaderModel4_0
            | HlslShaderModel::ShaderModel4_1
            | HlslShaderModel::ShaderModel5_0
            | HlslShaderModel::ShaderModel5_1 => {
                static SM4_0_LOWER: fn(&mut Builder) = |builder: &mut Builder| {
                    LowerLoopSampleLod::new(builder).do_pass();
                };
                Some(SM4_0_LOWER)
            }
            _ => None,
        };

        fn rewrite(words: &[u32], lowering_passes: fn(&mut Builder)) -> Vec<u32> {
            let mut builder = Builder::new_from_module(load_module(words));
            lowering_passes(&mut builder);
            builder.module().assemble()
        }

        if let Some(lowering_passes) = lowering_passes {
            let rewritten = SpirvCompilation {
                vertex: rewrite(&self.spirv.vertex, lowering_passes),
                fragment: rewrite(&self.spirv.fragment, lowering_passes),
            };

            let backend = HlslReflect::try_from(&rewritten).map_err(|e| match e {
                ShaderReflectError::SpirvCrossError(e) => {
                    ShaderCompileError::SpirvCrossCompileError(e)
                }
                e => ShaderCompileError::SpirvCrossCompileError(SpirvCrossError::InvalidArgument(
                    e.to_string(),
                )),
            })?;
            return backend.compile(options);
        }

        self.backend.compile(options)
    }

    fn compile_boxed(
        self: Box<Self>,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, Self::Context>, ShaderCompileError> {
        <Self as CompileShader<HLSL>>::compile(*self, options)
    }
}

impl CompileShader<HLSL> for CrossReflect<targets::Hlsl> {
    type Options = Option<HlslShaderModel>;
    type Context = CrossHlslContext;

    fn compile(
        mut self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, CrossHlslContext>, ShaderCompileError> {
        let sm = options.unwrap_or(HlslShaderModel::ShaderModel5_0);

        let mut options = targets::Hlsl::options();
        options.shader_model = sm;

        let vertex_resources = self.vertex.shader_resources()?;
        let fragment_resources = self.fragment.shader_resources()?;

        let mut vertex_buffer_assignment = HlslBufferAssignments::default();
        let mut fragment_buffer_assignment = HlslBufferAssignments::default();

        let mut vertex_ubo = vertex_resources.resources_for_type(ResourceType::UniformBuffer)?;
        if vertex_ubo.len() > 1 {
            return Err(ShaderCompileError::SpirvCrossCompileError(
                SpirvCrossError::InvalidArgument(String::from(
                    "Cannot have more than one uniform buffer",
                )),
            ));
        }

        if let Some(buf) = vertex_ubo.next() {
            vertex_buffer_assignment.ubo = Some(HlslBufferAssignment {
                name: buf.name.to_string(),
                id: buf.id.id(),
            })
        }

        let mut vertex_pcb = vertex_resources.resources_for_type(ResourceType::PushConstant)?;
        if vertex_pcb.len() > 1 {
            return Err(ShaderCompileError::SpirvCrossCompileError(
                SpirvCrossError::InvalidArgument(String::from(
                    "Cannot have more than one push constant buffer",
                )),
            ));
        }
        if let Some(buf) = vertex_pcb.next() {
            vertex_buffer_assignment.push = Some(HlslBufferAssignment {
                name: buf.name.to_string(),
                id: buf.id.id(),
            })
        }

        let mut fragment_ubo =
            fragment_resources.resources_for_type(ResourceType::UniformBuffer)?;
        if fragment_ubo.len() > 1 {
            return Err(ShaderCompileError::SpirvCrossCompileError(
                SpirvCrossError::InvalidArgument(String::from(
                    "Cannot have more than one uniform buffer",
                )),
            ));
        }

        if let Some(buf) = fragment_ubo.next() {
            fragment_buffer_assignment.ubo = Some(HlslBufferAssignment {
                name: buf.name.to_string(),
                id: buf.id.id(),
            })
        }

        let mut fragment_pcb = fragment_resources.resources_for_type(ResourceType::PushConstant)?;
        if fragment_pcb.len() > 1 {
            return Err(ShaderCompileError::SpirvCrossCompileError(
                SpirvCrossError::InvalidArgument(String::from(
                    "Cannot have more than one push constant buffer",
                )),
            ));
        }

        if let Some(buf) = fragment_pcb.next() {
            fragment_buffer_assignment.push = Some(HlslBufferAssignment {
                name: buf.name.to_string(),
                id: buf.id.id(),
            })
        }

        if sm == HlslShaderModel::ShaderModel3_0 {
            for res in fragment_resources.resources_for_type(ResourceType::SampledImage)? {
                let Some(DecorationValue::Literal(binding)) =
                    self.fragment.decoration(res.id, Decoration::Binding)?
                else {
                    continue;
                };
                self.fragment
                    .set_name(res.id, format!("LIBRA_SAMPLER2D_{binding}"))?;
                // self.fragment
                //     .unset_decoration(res.id, Decoration::Binding)?;
            }
        }

        let vertex_compiled = self.vertex.compile(&options)?;
        let fragment_compiled = self.fragment.compile(&options)?;

        Ok(ShaderCompilerOutput {
            vertex: vertex_compiled.to_string(),
            fragment: fragment_compiled.to_string(),
            context: CrossHlslContext {
                artifact: CompiledProgram {
                    vertex: vertex_compiled,
                    fragment: fragment_compiled,
                },

                vertex_buffers: vertex_buffer_assignment,
                fragment_buffers: fragment_buffer_assignment,
            },
        })
    }

    fn compile_boxed(
        self: Box<Self>,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, Self::Context>, ShaderCompileError> {
        <CrossReflect<targets::Hlsl> as CompileShader<HLSL>>::compile(*self, options)
    }
}
