use crate::error::ShaderCompileError;
use librashader_preprocess::ShaderSource;
use shaderc::{CompilationArtifact, CompileOptions, Limit, ShaderKind};

pub struct GlslangCompilation {
    pub(crate) vertex: CompilationArtifact,
    pub(crate) fragment: CompilationArtifact,
}

impl GlslangCompilation {

    /// Tries to compile SPIR-V from the provided shader source.
    pub fn compile(source: &ShaderSource) -> Result<Self, ShaderCompileError> {
        compile_spirv(source)
    }
}

impl TryFrom<&ShaderSource> for GlslangCompilation {
    type Error = ShaderCompileError;

    /// Tries to compile SPIR-V from the provided shader source.
    fn try_from(source: &ShaderSource) -> Result<Self, Self::Error> {
        GlslangCompilation::compile(source)
    }
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

fn compile_spirv(source: &ShaderSource) -> Result<GlslangCompilation, ShaderCompileError> {
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
    use crate::front::shaderc::compile_spirv;
    use librashader_preprocess::ShaderSource;
    #[test]
    pub fn compile_shader() {
        let result = ShaderSource::load("../test/basic.slang").unwrap();
        let _spirv = compile_spirv(&result).unwrap();
    }
}
