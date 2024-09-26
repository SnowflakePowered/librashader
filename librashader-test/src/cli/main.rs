use anyhow::anyhow;
use clap::{Parser, Subcommand};
use image::codecs::png::PngEncoder;
use librashader::presets::context::ContextItem;
use librashader::presets::{ShaderPreset, WildcardContext};
use librashader::reflect::cross::{GlslVersion, HlslShaderModel, MslVersion, SpirvCross};
use librashader::reflect::naga::{Naga, NagaLoweringOptions};
use librashader::reflect::semantics::ShaderSemantics;
use librashader::reflect::{CompileShader, FromCompilation, ReflectShader, SpirvCompilation};
use librashader_test::render::RenderTest;
use ron::ser::PrettyConfig;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Render a shader preset against an image
    Render {
        /// The frame to render.
        #[arg(short, long, default_value_t = 60)]
        frame: usize,
        /// The path to the shader preset to load.
        #[arg(short, long)]
        preset: PathBuf,
        /// The path to the input image.
        #[arg(short, long)]
        image: PathBuf,
        /// The path to the output image.
        ///
        /// If `-`, writes the image in PNG format to stdout.
        #[arg(short, long)]
        out: PathBuf,
        /// The runtime to use to render the shader preset.
        #[arg(value_enum, short, long)]
        runtime: Runtime,
    },
    /// Compare two runtimes and get a similarity score between the two
    /// runtimes rendering the same frame
    Compare {
        /// The frame to render.
        #[arg(short, long, default_value_t = 60)]
        frame: usize,
        /// The path to the shader preset to load.
        #[arg(short, long)]
        preset: PathBuf,
        /// The path to the input image.
        #[arg(short, long)]
        image: PathBuf,
        /// The runtime to compare against
        #[arg(value_enum, short, long)]
        left: Runtime,
        /// The runtime to compare to
        #[arg(value_enum, short, long)]
        right: Runtime,
        /// The path to write the similarity image.
        ///
        /// If `-`, writes the image to stdout.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Parse a preset and get a JSON representation of the data.
    Parse {
        /// The path to the shader preset to load.
        #[arg(short, long)]
        preset: PathBuf,
        /// Additional wildcard options, comma separated with equals signs. The PRESET and PRESET_DIR
        /// wildcards are always added to the preset parsing context.
        ///
        /// For example, CONTENT-DIR=MyVerticalGames,GAME=mspacman
        #[arg(short, long, value_delimiter = ',', num_args = 1..)]
        wildcards: Option<Vec<String>>,
    },
    /// Get the raw GLSL output of a preprocessed shader.
    Preprocess {
        /// The path to the slang shader.
        #[arg(short, long)]
        shader: PathBuf,
        /// The item to output.
        ///
        /// `json` will print a JSON representation of the preprocessed shader.
        #[arg(value_enum, short, long)]
        output: PreprocessOutput,
    },
    /// Transpile a shader in a given preset to the given format.
    Transpile {
        /// The path to the slang shader.
        #[arg(short, long)]
        shader: PathBuf,

        /// The shader stage to output.
        #[arg(value_enum, short = 'o', long)]
        stage: TranspileStage,

        /// The output format.
        #[arg(value_enum, short, long)]
        format: TranspileFormat,

        /// The version of the output format to parse as.
        /// This could be a GLSL version, a shader model, or an MSL version.
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Reflect the shader relative to a preset, giving information about semantics used in a slang shader.
    ///
    /// Due to limitations
    Reflect {
        /// The path to the shader preset to load.
        #[arg(short, long)]
        preset: PathBuf,
        /// Additional wildcard options, comma separated with equals signs. The PRESET and PRESET_DIR
        /// wildcards are always added to the preset parsing context.
        ///
        /// For example, CONTENT-DIR=MyVerticalGames,GAME=mspacman
        #[arg(short, long, value_delimiter = ',', num_args = 1..)]
        wildcards: Option<Vec<String>>,

        /// The pass index to use.
        #[arg(short, long)]
        index: usize,

        #[arg(value_enum, short, long, default_value = "cross")]
        backend: ReflectionBackend,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum PreprocessOutput {
    #[clap(name = "fragment")]
    Fragment,
    #[clap(name = "vertex")]
    Vertex,
    #[clap(name = "params")]
    Params,
    #[clap(name = "passformat")]
    Format,
    #[clap(name = "json")]
    Json,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum TranspileStage {
    #[clap(name = "fragment")]
    Fragment,
    #[clap(name = "vertex")]
    Vertex,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum TranspileFormat {
    #[clap(name = "glsl")]
    GLSL,
    #[clap(name = "hlsl")]
    HLSL,
    #[clap(name = "wgsl")]
    WGSL,
    #[clap(name = "msl")]
    MSL,
    #[clap(name = "spirv")]
    SPIRV,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Runtime {
    #[cfg(feature = "opengl")]
    #[clap(name = "opengl3")]
    OpenGL3,
    #[cfg(feature = "opengl")]
    #[clap(name = "opengl4")]
    OpenGL4,
    #[cfg(feature = "vulkan")]
    #[clap(name = "vulkan")]
    Vulkan,
    #[cfg(feature = "wgpu")]
    #[clap(name = "wgpu")]
    Wgpu,
    #[cfg(all(windows, feature = "d3d9"))]
    #[clap(name = "d3d9")]
    Direct3D9,
    #[cfg(all(windows, feature = "d3d11"))]
    #[clap(name = "d3d11")]
    Direct3D11,
    #[cfg(all(windows, feature = "d3d12"))]
    #[clap(name = "d3d12")]
    Direct3D12,
    #[cfg(all(target_vendor = "apple", feature = "metal"))]
    #[clap(name = "metal")]
    Metal,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ReflectionBackend {
    #[clap(name = "cross")]
    SpirvCross,
    #[clap(name = "naga")]
    Naga,
}

macro_rules! get_runtime {
    ($rt:ident, $image:ident) => {
        match $rt {
            #[cfg(feature = "opengl")]
            Runtime::OpenGL3 => &mut librashader_test::render::gl::OpenGl3::new($image.as_path())?,
            #[cfg(feature = "opengl")]
            Runtime::OpenGL4 => &mut librashader_test::render::gl::OpenGl4::new($image.as_path())?,
            #[cfg(feature = "vulkan")]
            Runtime::Vulkan => &mut librashader_test::render::vk::Vulkan::new($image.as_path())?,
            #[cfg(feature = "wgpu")]
            Runtime::Wgpu => &mut librashader_test::render::wgpu::Wgpu::new($image.as_path())?,
            #[cfg(all(windows, feature = "d3d9"))]
            Runtime::Direct3D9 => {
                &mut librashader_test::render::d3d9::Direct3D9::new($image.as_path())?
            }
            #[cfg(all(windows, feature = "d3d11"))]
            Runtime::Direct3D11 => {
                &mut librashader_test::render::d3d11::Direct3D11::new($image.as_path())?
            }
            #[cfg(all(windows, feature = "d3d12"))]
            Runtime::Direct3D12 => {
                &mut librashader_test::render::d3d12::Direct3D12::new($image.as_path())?
            }
            #[cfg(all(target_vendor = "apple", feature = "metal"))]
            Runtime::Metal => &mut librashader_test::render::mtl::Metal::new($image.as_path())?,
        }
    };
}
pub fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    match args.command {
        Commands::Render {
            frame,
            preset,
            image,
            out,
            runtime,
        } => {
            let test: &mut dyn RenderTest = get_runtime!(runtime, image);
            let image = test.render(preset.as_path(), frame)?;

            if out.as_path() == Path::new("-") {
                let out = std::io::stdout();
                image.write_with_encoder(PngEncoder::new(out))?;
            } else {
                image.save(out)?;
            }
        }
        Commands::Compare {
            frame,
            preset,
            image,
            left,
            right,
            out,
        } => {
            let left: &mut dyn RenderTest = get_runtime!(left, image);
            let right: &mut dyn RenderTest = get_runtime!(right, image);

            let left_image = left.render(preset.as_path(), frame)?;
            let right_image = right.render(preset.as_path(), frame)?;
            let similarity = image_compare::rgba_hybrid_compare(&left_image, &right_image)?;
            print!("{}", similarity.score);

            if let Some(out) = out {
                let image = similarity.image.to_color_map();
                if out.as_path() == Path::new("-") {
                    let out = std::io::stdout();
                    image.write_with_encoder(PngEncoder::new(out))?;
                } else {
                    image.save(out)?;
                }
            }
        }
        Commands::Parse { preset, wildcards } => {
            let preset = get_shader_preset(preset, wildcards)?;
            let out = serde_json::to_string_pretty(&preset)?;
            print!("{out:}");
        }
        Commands::Preprocess { shader, output } => {
            let source = librashader::preprocess::ShaderSource::load(shader.as_path())?;
            match output {
                PreprocessOutput::Fragment => print!("{}", source.fragment),
                PreprocessOutput::Vertex => print!("{}", source.vertex),
                PreprocessOutput::Params => {
                    print!("{}", serde_json::to_string_pretty(&source.parameters)?)
                }
                PreprocessOutput::Format => print!("{:?}", source.format),
                PreprocessOutput::Json => print!("{}", serde_json::to_string_pretty(&source)?),
            }
        }
        Commands::Transpile {
            shader,
            stage,
            format,
            version,
        } => {
            let source = librashader::preprocess::ShaderSource::load(shader.as_path())?;
            let compilation = SpirvCompilation::try_from(&source)?;
            let output = match format {
                TranspileFormat::GLSL => {
                    let mut compilation =
                        librashader::reflect::targets::GLSL::from_compilation(compilation)?;
                    compilation.validate()?;

                    let output = compilation.compile(GlslVersion::Glsl330)?;
                    TranspileOutput {
                        vertex: output.vertex,
                        fragment: output.fragment,
                    }
                }
                TranspileFormat::HLSL => {
                    let mut compilation =
                        librashader::reflect::targets::HLSL::from_compilation(compilation)?;
                    compilation.validate()?;
                    let output = compilation.compile(Some(HlslShaderModel::ShaderModel5_0))?;
                    TranspileOutput {
                        vertex: output.vertex,
                        fragment: output.fragment,
                    }
                }
                TranspileFormat::WGSL => {
                    let mut compilation =
                        librashader::reflect::targets::WGSL::from_compilation(compilation)?;
                    compilation.validate()?;
                    let output = compilation.compile(NagaLoweringOptions {
                        write_pcb_as_ubo: true,
                        sampler_bind_group: 1,
                    })?;
                    TranspileOutput {
                        vertex: output.vertex,
                        fragment: output.fragment,
                    }
                }
                TranspileFormat::MSL => {
                    let mut compilation = <librashader::reflect::targets::MSL as FromCompilation<
                        SpirvCompilation,
                        SpirvCross,
                    >>::from_compilation(compilation)?;
                    compilation.validate()?;
                    let output = compilation.compile(Some(MslVersion::new(1, 2, 0)))?;

                    TranspileOutput {
                        vertex: output.vertex,
                        fragment: output.fragment,
                    }
                }
                TranspileFormat::SPIRV => {
                    let mut compilation = <librashader::reflect::targets::SPIRV as FromCompilation<
                        SpirvCompilation,
                        SpirvCross,
                    >>::from_compilation(compilation)?;
                    compilation.validate()?;
                    let output = compilation.compile(None)?;

                    TranspileOutput {
                        vertex: spirv_to_dis(output.vertex)?,
                        fragment: spirv_to_dis(output.fragment)?,
                    }
                }
            };

            let print = match stage {
                TranspileStage::Fragment => output.fragment,
                TranspileStage::Vertex => output.vertex,
            };

            print!("{print}")
        }
        Commands::Reflect {
            preset,
            wildcards,
            index,
            backend,
        } => {
            let preset = get_shader_preset(preset, wildcards)?;
            let Some(shader) = preset.shaders.get(index) else {
                return Err(anyhow!("Invalid pass index for the preset"));
            };

            let source = librashader::preprocess::ShaderSource::load(shader.name.as_path())?;
            let compilation = SpirvCompilation::try_from(&source)?;

            let semantics =
                ShaderSemantics::create_pass_semantics::<anyhow::Error>(&preset, index)?;

            let reflection = match backend {
                ReflectionBackend::SpirvCross => {
                    let mut compilation =
                        <librashader::reflect::targets::SPIRV as FromCompilation<
                            SpirvCompilation,
                            SpirvCross,
                        >>::from_compilation(compilation)?;
                    compilation.reflect(index, &semantics)?
                }
                ReflectionBackend::Naga => {
                    let mut compilation =
                        <librashader::reflect::targets::SPIRV as FromCompilation<
                            SpirvCompilation,
                            Naga,
                        >>::from_compilation(compilation)?;
                    compilation.reflect(index, &semantics)?
                }
            };

            print!(
                "{}",
                ron::ser::to_string_pretty(&reflection, PrettyConfig::new())?
            );
        }
    }

    Ok(())
}

struct TranspileOutput {
    vertex: String,
    fragment: String,
}

fn get_shader_preset(
    preset: PathBuf,
    wildcards: Option<Vec<String>>,
) -> anyhow::Result<ShaderPreset> {
    let mut context = WildcardContext::new();
    context.add_path_defaults(preset.as_path());
    if let Some(wildcards) = wildcards {
        for string in wildcards {
            let Some((left, right)) = string.split_once("=") else {
                return Err(anyhow!("Encountered invalid context string {string}"));
            };

            context.append_item(ContextItem::ExternContext(
                left.to_string(),
                right.to_string(),
            ))
        }
    }
    let preset = ShaderPreset::try_parse_with_context(preset, context)?;
    Ok(preset)
}

fn spirv_to_dis(spirv: Vec<u32>) -> anyhow::Result<String> {
    let binary = spq_spvasm::SpirvBinary::from(spirv);
    spq_spvasm::Disassembler::new()
        .print_header(true)
        .name_ids(true)
        .name_type_ids(true)
        .name_const_ids(true)
        .indent(true)
        .disassemble(&binary)
}
