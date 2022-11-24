use std::error::Error;
use std::path::Path;
use rustc_hash::FxHashMap;
use windows::Win32::Graphics::Direct3D11::{D3D11_BIND_SHADER_RESOURCE, D3D11_RESOURCE_MISC_GENERATE_MIPS, D3D11_SAMPLER_DESC, D3D11_TEXTURE2D_DESC, ID3D11Device, ID3D11DeviceContext};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};
use librashader_common::image::Image;
use librashader_common::Size;
use librashader_preprocess::ShaderSource;
use librashader_presets::{ShaderPassConfig, ShaderPreset, TextureConfig};
use librashader_reflect::back::{CompilerBackend, CompileShader, FromCompilation};
use librashader_reflect::back::cross::GlslangHlslContext;
use librashader_reflect::back::targets::HLSL;
use librashader_reflect::front::shaderc::GlslangCompilation;
use librashader_reflect::reflect::ReflectShader;
use librashader_reflect::reflect::semantics::{ReflectSemantics, SemanticMap, TextureSemantics, UniformSemantic, VariableSemantics};
use crate::util::Texture;

type ShaderPassMeta<'a> = (
    &'a ShaderPassConfig,
    ShaderSource,
    CompilerBackend<
        impl CompileShader<HLSL, Options = Option<()>, Context = GlslangHlslContext> + ReflectShader,
    >,
);


struct FilterChain {

}

pub struct FilterCommon {
    pub(crate) device_context: ID3D11DeviceContext,
    pub(crate) preset: ShaderPreset,
}

// todo: d3d11.c 2097
type Result<T> = std::result::Result<T, Box<dyn Error>>;

impl FilterChain {
    fn load_pass_semantics(
        uniform_semantics: &mut FxHashMap<String, UniformSemantic>,
        texture_semantics: &mut FxHashMap<String, SemanticMap<TextureSemantics>>,
        config: &ShaderPassConfig,
    ) {
        let Some(alias) = &config.alias else {
            return;
        };

        // Ignore empty aliases
        if alias.trim().is_empty() {
            return;
        }

        let index = config.id as usize;

        // PassOutput
        texture_semantics.insert(
            alias.clone(),
            SemanticMap {
                semantics: TextureSemantics::PassOutput,
                index,
            },
        );
        uniform_semantics.insert(
            format!("{alias}Size"),
            UniformSemantic::Texture(SemanticMap {
                semantics: TextureSemantics::PassOutput,
                index,
            }),
        );

        // PassFeedback
        texture_semantics.insert(
            format!("{alias}Feedback"),
            SemanticMap {
                semantics: TextureSemantics::PassFeedback,
                index,
            },
        );
        uniform_semantics.insert(
            format!("{alias}FeedbackSize"),
            UniformSemantic::Texture(SemanticMap {
                semantics: TextureSemantics::PassFeedback,
                index,
            }),
        );
    }

    /// Load a filter chain from a pre-parsed `ShaderPreset`.
    pub fn load_from_preset(preset: ShaderPreset) -> Result<FilterChain> {
        let (passes, semantics) = FilterChain::load_preset(&preset)?;

        // initialize passes
        // let filters = FilterChain::init_passes(passes, &semantics)?;

        // let default_filter = filters.first().map(|f| f.config.filter).unwrap_or_default();
        // let default_wrap = filters
        //     .first()
        //     .map(|f| f.config.wrap_mode)
        //     .unwrap_or_default();

        // // initialize output framebuffers
        // let mut output_framebuffers = Vec::new();
        // output_framebuffers.resize_with(filters.len(), || Framebuffer::new(1));
        // let mut output_textures = Vec::new();
        // output_textures.resize_with(filters.len(), Texture::default);
        //
        // // initialize feedback framebuffers
        // let mut feedback_framebuffers = Vec::new();
        // feedback_framebuffers.resize_with(filters.len(), || Framebuffer::new(1));
        // let mut feedback_textures = Vec::new();
        // feedback_textures.resize_with(filters.len(), Texture::default);

        // load luts
        let luts = FilterChain::load_luts(&preset.textures)?;

        // let (history_framebuffers, history_textures) =
        //     FilterChain::init_history(&filters, default_filter, default_wrap);


        Ok(FilterChain {
            // passes: filters,
            // output_framebuffers: output_framebuffers.into_boxed_slice(),
            // feedback_framebuffers: feedback_framebuffers.into_boxed_slice(),
            // history_framebuffers,
            // filter_vao,
            // common: FilterCommon {
            //     // we don't need the reflect semantics once all locations have been bound per pass.
            //     // semantics,
            //     preset,
            //     luts,
            //     output_textures: output_textures.into_boxed_slice(),
            //     feedback_textures: feedback_textures.into_boxed_slice(),
            //     history_textures,
            //     draw_quad,
            // },
        })
    }

    fn load_luts(device: &ID3D11Device, textures: &[TextureConfig]) -> Result<FxHashMap<usize, Texture>> {
        let mut luts = FxHashMap::default();

        for (index, texture) in textures.iter().enumerate() {
            let image = Image::load(&texture.path)?;
            let desc = D3D11_TEXTURE2D_DESC {
                Width: image.width,
                Height: image.height,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                MiscFlags: if texture.mipmap {
                    D3D11_RESOURCE_MISC_GENERATE_MIPS
                } else {
                    0
                },
                ..Default::default()
            };

            let mut texture = Texture::new(device, image.size, desc);
            // todo: update texture d3d11_common: 150
            luts.insert(index, texture);

        }
        Ok(luts)
    }

    /// Load the shader preset at the given path into a filter chain.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<FilterChain> {
        // load passes from preset
        let preset = ShaderPreset::try_parse(path)?;
        Self::load_from_preset(preset)
    }

    fn load_preset(
        preset: &ShaderPreset,
    ) -> Result<(Vec<ShaderPassMeta>, ReflectSemantics)> {
        let mut uniform_semantics: FxHashMap<String, UniformSemantic> = Default::default();
        let mut texture_semantics: FxHashMap<String, SemanticMap<TextureSemantics>> =
            Default::default();

        let passes = preset
            .shaders
            .iter()
            .map(|shader| {
                eprintln!("[dx11] loading {}", &shader.name.display());
                let source: ShaderSource = ShaderSource::load(&shader.name)?;

                let spirv = GlslangCompilation::compile(&source)?;
                let reflect = HLSL::from_compilation(spirv)?;

                for parameter in source.parameters.iter() {
                    uniform_semantics.insert(
                        parameter.id.clone(),
                        UniformSemantic::Variable(SemanticMap {
                            semantics: VariableSemantics::FloatParameter,
                            index: (),
                        }),
                    );
                }
                Ok::<_, Box<dyn Error>>((shader, source, reflect))
            })
            .into_iter()
            .collect::<Result<Vec<(&ShaderPassConfig, ShaderSource, CompilerBackend<_>)>>>()?;

        for details in &passes {
            FilterChain::load_pass_semantics(
                &mut uniform_semantics,
                &mut texture_semantics,
                details.0,
            )
        }

        // add lut params
        for (index, texture) in preset.textures.iter().enumerate() {
            texture_semantics.insert(
                texture.name.clone(),
                SemanticMap {
                    semantics: TextureSemantics::User,
                    index,
                },
            );

            uniform_semantics.insert(
                format!("{}Size", texture.name),
                UniformSemantic::Texture(SemanticMap {
                    semantics: TextureSemantics::User,
                    index,
                }),
            );
        }

        let semantics = ReflectSemantics {
            uniform_semantics,
            non_uniform_semantics: texture_semantics,
        };

        Ok((passes, semantics))
    }
}