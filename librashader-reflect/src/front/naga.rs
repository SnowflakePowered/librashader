use crate::error::ShaderCompileError;
use librashader_preprocess::ShaderSource;
use naga::front::glsl::{Options, Parser};
use naga::{Module, ShaderStage};

#[derive(Debug)]
pub struct NagaCompilation {
    pub(crate) vertex: Module,
    pub(crate) fragment: Module,
}

pub fn compile_spirv(source: &ShaderSource) -> Result<NagaCompilation, ShaderCompileError> {
    let mut parser = Parser::default();
    let vertex = parser.parse(&Options::from(ShaderStage::Vertex), &source.vertex)?;
    let fragment = parser.parse(&Options::from(ShaderStage::Fragment), &source.fragment)?;
    Ok(NagaCompilation { vertex, fragment })
}

#[cfg(test)]
mod test {
    use crate::front::naga::compile_spirv;
    use naga::back::glsl::{PipelineOptions, Version};
    use naga::back::spv::{Capability, WriterFlags};
    use naga::front::glsl::{Options, Parser};
    use naga::front::spv::Options as SpvOptions;
    use naga::valid::{Capabilities, ValidationFlags};
    use naga::{FastHashSet, ShaderStage};
    use rspirv::binary::Disassemble;
    use librashader_preprocess::ShaderSource;
    use crate::front::shaderc::GlslangCompilation;

    #[test]
    pub fn compile_naga_test() {
        let result = ShaderSource::load(
            "../test/slang-shaders/blurs/shaders/royale/blur3x3-last-pass.slang",
        )
        .unwrap();

        let fragment_source = result.fragment;
        let mut parser = Parser::default();
        println!("{fragment_source}");
        let _fragment = parser
            .parse(&Options::from(ShaderStage::Fragment), &fragment_source)
            .unwrap();
    }

    #[test]
    pub fn compile_shader() {
        let result =  ShaderSource::load(
            "../test/slang-shaders/blurs/shaders/royale/blur3x3-last-pass.slang",
        )
        .unwrap();
        let spirv = compile_spirv(&result).unwrap();
        eprintln!("{spirv:?}")
    }

    #[test]
    pub fn compile_shader_roundtrip() {
        let result =  ShaderSource::load("../test/basic.slang").unwrap();
        let cross = GlslangCompilation::compile(&result).unwrap();
        let naga_fragment =
            naga::front::spv::parse_u8_slice(cross.fragment.as_binary_u8(), &SpvOptions::default())
                .unwrap();
        println!("{:#?}", naga_fragment.constants);
        println!("{:#?}", naga_fragment.global_variables);
        println!("{:#?}", naga_fragment.types);
    }

    #[test]
    pub fn naga_playground() {
        let result = ShaderSource::load("../test/basic.slang").unwrap();
        let spirv = GlslangCompilation::compile(&result).unwrap();

        let module =
            naga::front::spv::parse_u8_slice(spirv.fragment.as_binary_u8(), &SpvOptions::default())
                .unwrap();

        let capability = FastHashSet::from_iter([Capability::Shader]);
        let mut writer = naga::back::spv::Writer::new(&naga::back::spv::Options {
            lang_version: (1, 0),
            flags: WriterFlags::all(),
            binding_map: Default::default(),
            capabilities: Some(capability),
            bounds_check_policies: Default::default(),
        })
        .unwrap();

        let mut validator =
            naga::valid::Validator::new(ValidationFlags::empty(), Capabilities::all());
        let info = validator.validate(&module).unwrap();
        let mut spv_out = Vec::new();
        let pipe = naga::back::spv::PipelineOptions {
            shader_stage: ShaderStage::Fragment,
            entry_point: "main".to_string(),
        };
        writer
            .write(&module, &info, Some(&pipe), &mut spv_out)
            .unwrap();

        let mut glsl_out = String::new();
        let opts = naga::back::glsl::Options {
            version: Version::Desktop(330),
            writer_flags: naga::back::glsl::WriterFlags::all(),
            binding_map: Default::default(),
        };
        let pipe = PipelineOptions {
            shader_stage: ShaderStage::Fragment,
            entry_point: "main".to_string(),
            multiview: None,
        };
        let mut glsl_naga = naga::back::glsl::Writer::new(
            &mut glsl_out,
            &module,
            &info,
            &opts,
            &pipe,
            Default::default(),
        )
        .unwrap();

        glsl_naga.write().unwrap();

        let wgsl =
            naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::all())
                .unwrap();

        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_words(&spv_out, &mut loader).unwrap();
        let module = loader.module();
        println!("--- spirv --");
        println!("{:#}", module.disassemble());
        println!("--- cross glsl --");

        let loaded = spirv_cross::spirv::Module::from_words(&spv_out);
        let mut ast = spirv_cross::spirv::Ast::<spirv_cross::glsl::Target>::parse(&loaded).unwrap();
        println!("{:#}", ast.compile().unwrap());
        println!("--- naga glsl---");
        println!("{glsl_out:#}");
        println!("--- naga wgsl---");
        println!("{wgsl:#}")
    }
}
