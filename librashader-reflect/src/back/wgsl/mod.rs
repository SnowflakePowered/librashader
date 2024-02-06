mod lower_samplers;

use crate::back::targets::WGSL;
use crate::back::wgsl::lower_samplers::LowerCombinedImageSamplerPass;
use crate::back::{CompileShader, CompilerBackend, FromCompilation, ShaderCompilerOutput};
use crate::error::{ShaderCompileError, ShaderReflectError};
use crate::front::GlslangCompilation;
use crate::reflect::naga::NagaReflect;
use crate::reflect::ReflectShader;
use naga::back::wgsl::WriterFlags;
use naga::valid::{Capabilities, ValidationFlags};
use naga::{AddressSpace, Module};
use rspirv::binary::Assemble;
use rspirv::dr::Builder;

/// The context for a WGSL compilation via Naga
pub struct NagaWgslContext {
    pub fragment: Module,
    pub vertex: Module,
}

/// Compiler options for WGSL
#[derive(Debug, Default, Clone)]
pub struct WgslCompileOptions {
    pub write_pcb_as_ubo: bool,
    pub sampler_bind_group: u32,
}

impl FromCompilation<GlslangCompilation> for WGSL {
    type Target = WGSL;
    type Options = WgslCompileOptions;
    type Context = NagaWgslContext;
    type Output = impl CompileShader<Self::Target, Options = Self::Options, Context = Self::Context>
        + ReflectShader;

    fn from_compilation(
        compile: GlslangCompilation,
    ) -> Result<CompilerBackend<Self::Output>, ShaderReflectError> {
        fn lower_fragment_shader(words: &[u32]) -> Vec<u32> {
            let mut loader = rspirv::dr::Loader::new();
            rspirv::binary::parse_words(words, &mut loader).unwrap();
            let module = loader.module();
            let mut builder = Builder::new_from_module(module);

            let mut pass = LowerCombinedImageSamplerPass::new(&mut builder);

            pass.ensure_op_type_sampler();
            pass.do_pass();

            let module = builder.module();

            module.assemble()
        }

        let options = naga::front::spv::Options {
            adjust_coordinate_space: true,
            strict_capabilities: false,
            block_ctx_dump_prefix: None,
        };

        let vertex =
            naga::front::spv::parse_u8_slice(bytemuck::cast_slice(&compile.vertex), &options)?;

        let fragment = lower_fragment_shader(&compile.fragment);
        let fragment = naga::front::spv::parse_u8_slice(bytemuck::cast_slice(&fragment), &options)?;

        Ok(CompilerBackend {
            backend: NagaReflect { vertex, fragment },
        })
    }
}

impl CompileShader<WGSL> for NagaReflect {
    type Options = WgslCompileOptions;
    type Context = NagaWgslContext;

    fn compile(
        mut self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, Self::Context>, ShaderCompileError> {
        fn write_wgsl(module: &Module) -> Result<String, ShaderCompileError> {
            let mut valid =
                naga::valid::Validator::new(ValidationFlags::all(), Capabilities::empty());
            let info = valid.validate(&module)?;

            let wgsl = naga::back::wgsl::write_string(&module, &info, WriterFlags::EXPLICIT_TYPES)?;
            Ok(wgsl)
        }

        if options.write_pcb_as_ubo {
            for (_, gv) in self.fragment.global_variables.iter_mut() {
                if gv.space == AddressSpace::PushConstant {
                    gv.space = AddressSpace::Uniform;
                }
            }

            for (_, gv) in self.vertex.global_variables.iter_mut() {
                if gv.space == AddressSpace::PushConstant {
                    gv.space = AddressSpace::Uniform;
                }
            }
        } else {
            for (_, gv) in self.fragment.global_variables.iter_mut() {
                if gv.space == AddressSpace::PushConstant {
                    gv.binding = None;
                }
            }
        }

        // Reassign shit.
        let images = self
            .fragment
            .global_variables
            .iter()
            .filter(|&(_, gv)| {
                let ty = &self.fragment.types[gv.ty];
                match ty.inner {
                    naga::TypeInner::Image { .. } => true,
                    naga::TypeInner::BindingArray { base, .. } => {
                        let ty = &self.fragment.types[base];
                        matches!(ty.inner, naga::TypeInner::Image { .. })
                    }
                    _ => false,
                }
            })
            .map(|(_, gv)| (gv.binding.clone(), gv.space))
            .collect::<naga::FastHashSet<_>>();

        self.fragment
            .global_variables
            .iter_mut()
            .filter(|(_, gv)| {
                let ty = &self.fragment.types[gv.ty];
                match ty.inner {
                    naga::TypeInner::Sampler { .. } => true,
                    naga::TypeInner::BindingArray { base, .. } => {
                        let ty = &self.fragment.types[base];
                        matches!(ty.inner, naga::TypeInner::Sampler { .. })
                    }
                    _ => false,
                }
            })
            .for_each(|(_, gv)| {
                if images.contains(&(gv.binding.clone(), gv.space)) {
                    if let Some(binding) = &mut gv.binding {
                        binding.group = options.sampler_bind_group;
                    }
                }
            });

        let fragment = write_wgsl(&self.fragment)?;
        let vertex = write_wgsl(&self.vertex)?;
        Ok(ShaderCompilerOutput {
            vertex,
            fragment,
            context: NagaWgslContext {
                fragment: self.fragment,
                vertex: self.vertex,
            },
        })
    }
}

#[cfg(test)]
mod test {
    use crate::back::targets::WGSL;
    use crate::back::wgsl::WgslCompileOptions;
    use crate::back::{CompileShader, FromCompilation};
    use crate::reflect::semantics::{Semantic, ShaderSemantics, UniformSemantic, UniqueSemantics};
    use crate::reflect::ReflectShader;
    use librashader_preprocess::ShaderSource;
    use rustc_hash::FxHashMap;

    #[test]
    pub fn test_into() {
        // let result = ShaderSource::load("../test/shaders_slang/crt/shaders/crt-royale/src/crt-royale-scanlines-horizontal-apply-mask.slang").unwrap();
        // let result = ShaderSource::load("../test/shaders_slang/crt/shaders/crt-royale/src/crt-royale-scanlines-horizontal-apply-mask.slang").unwrap();
        let result = ShaderSource::load("../test/basic.slang").unwrap();

        let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();

        for (_index, param) in result.parameters.iter().enumerate() {
            uniform_semantics.insert(
                param.1.id.clone(),
                UniformSemantic::Unique(Semantic {
                    semantics: UniqueSemantics::FloatParameter,
                    index: (),
                }),
            );
        }

        let compilation = crate::front::GlslangCompilation::try_from(&result).unwrap();

        let mut wgsl = WGSL::from_compilation(compilation).unwrap();

        wgsl.reflect(
            0,
            &ShaderSemantics {
                uniform_semantics,
                texture_semantics: Default::default(),
            },
        )
        .expect("");

        let compiled = wgsl
            .compile(WgslCompileOptions {
                write_pcb_as_ubo: true,
                sampler_bind_group: 1,
            })
            .unwrap();

        println!("{}", compiled.fragment);

        // println!("{}", compiled.fragment);
        // let mut loader = rspirv::dr::Loader::new();
        // rspirv::binary::parse_words(compilation.vertex.as_binary(), &mut loader).unwrap();
        // let module = loader.module();
        //
        // let outputs: Vec<&Instruction> = module
        //     .types_global_values
        //     .iter()
        //     .filter(|i| i.class.opcode == Op::Variable)
        //     .collect();
        //
        // println!("{outputs:#?}");
    }
}
