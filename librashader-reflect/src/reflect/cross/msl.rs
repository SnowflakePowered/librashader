use std::collections::BTreeMap;
use naga::{Module};
use crate::back::msl::CrossMslContext;
use crate::back::targets::MSL;
use crate::back::{CompileShader, ShaderCompilerOutput};
use crate::error::ShaderCompileError;
use crate::reflect::cross::{CompiledAst, CompiledProgram, CrossReflect};
use spirv_cross::msl;
use spirv_cross::msl::{ResourceBinding, ResourceBindingLocation};
use spirv_cross::spirv::{Ast, Decoration, ExecutionModel};

pub(crate) type MslReflect = CrossReflect<spirv_cross::msl::Target>;

impl CompileShader<MSL> for CrossReflect<spirv_cross::msl::Target> {
    type Options = Option<spirv_cross::msl::Version>;
    type Context = CrossMslContext;

    fn compile(
        mut self,
        options: Self::Options,
    ) -> Result<ShaderCompilerOutput<String, CrossMslContext>, ShaderCompileError> {
        let version = options.unwrap_or(msl::Version::V2_0);
        let mut vert_options = spirv_cross::msl::CompilerOptions::default();
        let mut frag_options = spirv_cross::msl::CompilerOptions::default();

        vert_options.version = version;
        frag_options.version = version;

        fn get_binding(ast: &Ast<msl::Target>, stage: ExecutionModel, binding_map: &mut BTreeMap<ResourceBindingLocation, ResourceBinding>) -> Result<(), ShaderCompileError>{
            let resources = ast.get_shader_resources()?;
            for resource in &resources.push_constant_buffers {
                let location = ResourceBindingLocation {
                    stage,
                    desc_set: u32::MAX, // ResourceBindingPushConstantDescriptorSet
                    binding: 0,
                };

                let overridden = ResourceBinding {
                    buffer_id: ast.get_decoration(resource.id, Decoration::Binding)?,
                    texture_id: 0,
                    sampler_id: 0,
                    count: 0,
                };

                binding_map.insert(location, overridden);
            }

            for resource in resources.uniform_buffers.iter().chain(resources.sampled_images.iter()) {
                let binding = ast.get_decoration(resource.id, Decoration::Binding)?;
                let location = ResourceBindingLocation {
                    stage,
                    desc_set: ast.get_decoration(resource.id, Decoration::DescriptorSet)?,
                    binding,
                };

                let overridden = ResourceBinding {
                    buffer_id: binding,
                    texture_id: binding,
                    sampler_id: binding,
                    count: 0,
                };

                binding_map.insert(location, overridden);
            }

            Ok(())
        }
        get_binding(
            &self.vertex,
            ExecutionModel::Vertex,
            &mut vert_options.resource_binding_overrides
        )?;

        get_binding(
            &self.fragment,
            ExecutionModel::Fragment,
            &mut frag_options.resource_binding_overrides
        )?;

        self.vertex.set_compiler_options(&vert_options)?;
        self.fragment.set_compiler_options(&frag_options)?;

        Ok(ShaderCompilerOutput {
            vertex: self.vertex.compile()?,
            fragment: self.fragment.compile()?,
            context: CrossMslContext {
                artifact: CompiledProgram {
                    vertex: CompiledAst(self.vertex),
                    fragment: CompiledAst(self.fragment),
                },
            },
        })
    }
}

#[cfg(test)]
mod test {
    use crate::back::targets::{MSL, WGSL};
    use crate::back::{CompileShader, FromCompilation};
    use crate::reflect::naga::{Naga, NagaLoweringOptions};
    use crate::reflect::semantics::{Semantic, ShaderSemantics, UniformSemantic, UniqueSemantics};
    use crate::reflect::ReflectShader;
    use librashader_preprocess::ShaderSource;
    use rustc_hash::FxHashMap;
    use bitflags::Flags;
    use spirv_cross::msl;
    use crate::reflect::cross::SpirvCross;

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

        let compilation = crate::front::SpirvCompilation::try_from(&result).unwrap();

        let mut msl = <MSL as FromCompilation<_, SpirvCross>>::from_compilation(compilation).unwrap();

        msl.reflect(
            0,
            &ShaderSemantics {
                uniform_semantics,
                texture_semantics: Default::default(),
            },
        )
            .expect("");

        let compiled = msl
            .compile(Some(msl::Version::V2_0))
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