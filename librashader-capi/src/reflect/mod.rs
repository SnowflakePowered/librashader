use crate::error;
use librashader::preprocess::ShaderSource;
use librashader::presets::{ShaderPassConfig, ShaderPreset, TextureConfig};
use librashader::reflect::image::{Image, UVDirection, RGBA8};
use librashader::reflect::semantics::{
    Semantic, ShaderSemantics, TextureSemantics, UniformSemantic, UniqueSemantics,
};
use librashader::reflect::targets::SpirV;
use librashader::reflect::{
    CompileShader, CompilerBackend, FromCompilation, GlslangCompilation, ReflectShader,
    ShaderCompilerOutput, ShaderReflection,
};
use librashader::{FilterMode, WrapMode};
use rustc_hash::FxHashMap;

pub(crate) struct LookupTexture {
    wrap_mode: WrapMode,
    /// The filter mode to use when sampling the texture.
    filter_mode: FilterMode,
    /// Whether or not to generate mipmaps for this texture.
    mipmap: bool,
    /// The image data of the texture
    image: Image,
}

pub(crate) struct PassReflection {
    reflection: ShaderReflection,
    config: ShaderPassConfig,
    spirv: ShaderCompilerOutput<Vec<u32>>,
}
pub(crate) struct FilterReflection {
    semantics: ShaderSemantics,
    passes: Vec<PassReflection>,
    textures: Vec<LookupTexture>,
}

impl FilterReflection {
    pub fn load_from_preset(
        preset: ShaderPreset,
        direction: UVDirection,
    ) -> Result<FilterReflection, error::LibrashaderError> {
        let (passes, textures) = (preset.shaders, preset.textures);
        let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();
        let mut texture_semantics: FxHashMap<String, Semantic<TextureSemantics>> =
            Default::default();

        let passes = passes
            .into_iter()
            .enumerate()
            .map(|(index, shader)| {
                let source: ShaderSource = ShaderSource::load(&shader.name)?;

                let spirv = GlslangCompilation::compile(&source)?;
                let mut reflect = SpirV::from_compilation(spirv)?;

                for parameter in source.parameters.iter() {
                    uniform_semantics.insert(
                        parameter.id.clone(),
                        UniformSemantic::Unique(Semantic {
                            semantics: UniqueSemantics::FloatParameter,
                            index: (),
                        }),
                    );
                }

                Ok::<_, error::LibrashaderError>((shader, source, reflect))
            })
            .into_iter()
            .collect::<Result<
                Vec<(ShaderPassConfig, ShaderSource, CompilerBackend<_>)>,
                error::LibrashaderError,
            >>()?;

        for details in &passes {
            librashader::runtime::helper::insert_pass_semantics(
                &mut uniform_semantics,
                &mut texture_semantics,
                &details.0,
            )
        }

        librashader::runtime::helper::insert_lut_semantics(
            &textures,
            &mut uniform_semantics,
            &mut texture_semantics,
        );

        let semantics = ShaderSemantics {
            uniform_semantics,
            texture_semantics,
        };

        let mut reflects = Vec::new();

        for (index, (config, _source, mut compiler)) in passes.into_iter().enumerate() {
            let reflection = compiler.reflect(index, &semantics)?;
            let words = compiler.compile(None)?;
            reflects.push(PassReflection {
                reflection,
                config,
                spirv: words,
            })
        }

        let textures = textures
            .into_iter()
            .map(|texture| {
                let lut = Image::<RGBA8>::load(&texture.path, direction)
                    .map_err(|e| error::LibrashaderError::UnknownError(Box::new(e)))?;
                Ok(LookupTexture {
                    wrap_mode: texture.wrap_mode,
                    filter_mode: texture.filter_mode,
                    mipmap: texture.mipmap,
                    image: lut,
                })
            })
            .into_iter()
            .collect::<Result<Vec<LookupTexture>, error::LibrashaderError>>()?;

        Ok(FilterReflection {
            semantics,
            passes: reflects,
            textures,
        })
    }
}
