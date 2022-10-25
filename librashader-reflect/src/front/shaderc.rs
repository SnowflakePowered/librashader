use crate::error::ShaderCompileError;
use librashader::ShaderSource;
use shaderc::{CompilationArtifact, CompileOptions, GlslProfile, Limit, ShaderKind};

pub struct GlslangCompilation {
    pub(crate) vertex: CompilationArtifact,
    pub(crate) fragment: CompilationArtifact,
}

fn get_shaderc_options() -> Result<CompileOptions<'static>, ShaderCompileError> {
    let mut options = CompileOptions::new().ok_or(ShaderCompileError::ShaderCInitError)?;
    options.set_include_callback(|_, _, _, _| {
        Err("RetroArch shaders must already have includes be preprocessed".into())
    });
    options.set_limit(Limit::MaxLights, 32);
    options.set_limit(Limit::MaxClipPlanes, 6);
    options.set_limit(Limit::MaxTextureUnits, 32);
    options.set_limit(Limit::MaxTextureCoords, 32);
    options.set_limit(Limit::MaxVertexAttribs, 64);
    options.set_limit(Limit::MaxVertexUniformComponents, 4096);
    options.set_limit(Limit::MaxVaryingFloats, 64);
    options.set_limit(Limit::MaxVertexTextureImageUnits, 32);
    options.set_limit(Limit::MaxCombinedTextureImageUnits, 80);
    options.set_limit(Limit::MaxTextureImageUnits, 32);
    options.set_limit(Limit::MaxFragmentUniformComponents, 4096);
    options.set_limit(Limit::MaxDrawBuffers, 32);
    options.set_limit(Limit::MaxVertexUniformVectors, 128);
    options.set_limit(Limit::MaxVaryingVectors, 8);
    options.set_limit(Limit::MaxFragmentUniformVectors, 16);
    options.set_limit(Limit::MaxVertexOutputVectors, 16);
    options.set_limit(Limit::MaxFragmentInputVectors, 15);
    options.set_limit(Limit::MinProgramTexelOffset, -8);
    options.set_limit(Limit::MaxProgramTexelOffset, 7);
    options.set_limit(Limit::MaxClipDistances, 8);
    options.set_limit(Limit::MaxComputeWorkGroupCountX, 65535);
    options.set_limit(Limit::MaxComputeWorkGroupCountY, 65535);
    options.set_limit(Limit::MaxComputeWorkGroupCountZ, 65535);
    options.set_limit(Limit::MaxComputeWorkGroupSizeX, 1024);
    options.set_limit(Limit::MaxComputeWorkGroupSizeY, 1024);
    options.set_limit(Limit::MaxComputeWorkGroupSizeZ, 64);
    options.set_limit(Limit::MaxComputeUniformComponents, 1024);
    options.set_limit(Limit::MaxComputeTextureImageUnits, 16);
    options.set_limit(Limit::MaxComputeImageUniforms, 8);
    options.set_limit(Limit::MaxComputeAtomicCounters, 8);
    options.set_limit(Limit::MaxComputeAtomicCounterBuffers, 1);
    options.set_limit(Limit::MaxVaryingComponents, 60);
    options.set_limit(Limit::MaxVertexOutputComponents, 64);
    options.set_limit(Limit::MaxGeometryInputComponents, 64);
    options.set_limit(Limit::MaxGeometryOutputComponents, 128);
    options.set_limit(Limit::MaxFragmentInputComponents, 128);
    options.set_limit(Limit::MaxImageUnits, 8);
    options.set_limit(Limit::MaxCombinedImageUnitsAndFragmentOutputs, 8);
    options.set_limit(Limit::MaxCombinedShaderOutputResources, 8);
    options.set_limit(Limit::MaxImageSamples, 0);
    options.set_limit(Limit::MaxVertexImageUniforms, 0);
    options.set_limit(Limit::MaxTessControlImageUniforms, 0);
    options.set_limit(Limit::MaxTessEvaluationImageUniforms, 0);
    options.set_limit(Limit::MaxGeometryImageUniforms, 0);
    options.set_limit(Limit::MaxFragmentImageUniforms, 8);
    options.set_limit(Limit::MaxCombinedImageUniforms, 8);
    options.set_limit(Limit::MaxGeometryTextureImageUnits, 16);
    options.set_limit(Limit::MaxGeometryOutputVertices, 256);
    options.set_limit(Limit::MaxGeometryTotalOutputComponents, 1024);
    options.set_limit(Limit::MaxGeometryUniformComponents, 1024);
    options.set_limit(Limit::MaxGeometryVaryingComponents, 64);
    options.set_limit(Limit::MaxTessControlInputComponents, 128);
    options.set_limit(Limit::MaxTessControlOutputComponents, 128);
    options.set_limit(Limit::MaxTessControlTextureImageUnits, 16);
    options.set_limit(Limit::MaxTessControlUniformComponents, 1024);
    options.set_limit(Limit::MaxTessControlTotalOutputComponents, 4096);
    options.set_limit(Limit::MaxTessEvaluationInputComponents, 128);
    options.set_limit(Limit::MaxTessEvaluationOutputComponents, 128);
    options.set_limit(Limit::MaxTessEvaluationTextureImageUnits, 16);
    options.set_limit(Limit::MaxTessEvaluationUniformComponents, 1024);
    options.set_limit(Limit::MaxTessPatchComponents, 120);
    options.set_limit(Limit::MaxPatchVertices, 32);
    options.set_limit(Limit::MaxTessGenLevel, 64);
    options.set_limit(Limit::MaxViewports, 16);
    options.set_limit(Limit::MaxVertexAtomicCounters, 0);
    options.set_limit(Limit::MaxTessControlAtomicCounters, 0);
    options.set_limit(Limit::MaxTessEvaluationAtomicCounters, 0);
    options.set_limit(Limit::MaxGeometryAtomicCounters, 0);
    options.set_limit(Limit::MaxFragmentAtomicCounters, 8);
    options.set_limit(Limit::MaxCombinedAtomicCounters, 8);
    options.set_limit(Limit::MaxAtomicCounterBindings, 1);
    options.set_limit(Limit::MaxVertexAtomicCounterBuffers, 0);
    options.set_limit(Limit::MaxTessControlAtomicCounterBuffers, 0);
    options.set_limit(Limit::MaxTessEvaluationAtomicCounterBuffers, 0);
    options.set_limit(Limit::MaxGeometryAtomicCounterBuffers, 0);
    options.set_limit(Limit::MaxFragmentAtomicCounterBuffers, 1);
    options.set_limit(Limit::MaxCombinedAtomicCounterBuffers, 1);
    options.set_limit(Limit::MaxAtomicCounterBufferSize, 16384);
    options.set_limit(Limit::MaxTransformFeedbackBuffers, 4);
    options.set_limit(Limit::MaxTransformFeedbackInterleavedComponents, 64);
    options.set_limit(Limit::MaxCullDistances, 8);
    options.set_limit(Limit::MaxCombinedClipAndCullDistances, 8);
    options.set_limit(Limit::MaxSamples, 4);

    Ok(options)
}

pub fn compile_spirv(source: &ShaderSource) -> Result<GlslangCompilation, ShaderCompileError> {
    let compiler = shaderc::Compiler::new().ok_or(ShaderCompileError::ShaderCInitError)?;
    let name = source.name.as_deref().unwrap_or("shader.slang");
    let options = get_shaderc_options()?;

    let vertex = compiler.compile_into_spirv(
        &source.vertex,
        ShaderKind::Vertex,
        name,
        "main",
        Some(&options),
    )?;
    let fragment = compiler.compile_into_spirv(
        &source.fragment,
        ShaderKind::Fragment,
        name,
        "main",
        Some(&options),
    )?;
    Ok(GlslangCompilation { vertex, fragment })
}

#[cfg(test)]
mod test {
    use rspirv::binary::Disassemble;
    use naga::back::glsl::{PipelineOptions, Version};
    use naga::back::spv::{Capability, WriterFlags};
    use naga::{FastHashSet, ShaderStage};
    use naga::front::spv::Options;
    use naga::valid::{Capabilities, ModuleInfo, ValidationFlags};
    use crate::front::shaderc::compile_spirv;

    #[test]
    pub fn compile_shader() {
        let result = librashader_preprocess::load_shader_source(
            "../test/basic.slang",
        )
            .unwrap();
        let spirv = compile_spirv(&result).unwrap();
    }

    #[test]
    pub fn naga_playground() {
        let result = librashader_preprocess::load_shader_source(
            "../test/basic.slang",
        )
        .unwrap();
        let spirv = compile_spirv(&result).unwrap();

        let module = naga::front::spv::parse_u8_slice(spirv.fragment.as_binary_u8(), &Options {
            adjust_coordinate_space: false,
            strict_capabilities: false,
            block_ctx_dump_prefix: None
        }).unwrap();

        let capability = FastHashSet::from_iter([Capability::Shader]);
        let mut writer = naga::back::spv::Writer::new(&naga::back::spv::Options {
            lang_version: (1, 0),
            flags: WriterFlags::all(),
            binding_map: Default::default(),
            capabilities: Some(capability),
            bounds_check_policies: Default::default()
        }).unwrap();

        let mut validator = naga::valid::Validator::new(ValidationFlags::empty(), Capabilities::all());
        let info = validator.validate(&module).unwrap();
        let mut out = Vec::new();
        writer.write(&module, &info, None, &mut out).unwrap();

        let mut glsl_out = String::new();
        let opts = naga::back::glsl::Options {
            version: Version::Desktop(330),
            writer_flags: naga::back::glsl::WriterFlags::all(),
            binding_map: Default::default()
        };
        let pipe = PipelineOptions {
            shader_stage: ShaderStage::Fragment,
            entry_point: "main".to_string(),
            multiview: None
        };
        let mut glsl_naga = naga::back::glsl::Writer::new(&mut glsl_out, &module, &info, &opts, &pipe, Default::default()).unwrap();

        glsl_naga.write().unwrap();

        let wgsl = naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::all()).unwrap();

        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_words(&out, &mut loader).unwrap();
        let module = loader.module();
        println!("--- spirv --");
        println!("{:#}", module.disassemble());
        println!("--- cross glsl --");

        let loaded = spirv_cross::spirv::Module::from_words(&out);
        let mut ast = spirv_cross::spirv::Ast::<spirv_cross::glsl::Target>::parse(&loaded)
            .unwrap();
        println!("{:#}", ast.compile().unwrap());
        println!("--- naga glsl---");
        println!("{:#}", glsl_out);
        println!("--- naga wgsl---");
        println!("{:#}", wgsl)
    }
}
