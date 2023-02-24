use crate::error::ParsePresetError;
use nom::branch::alt;
use nom::bytes::complete::{is_not, take_until};
use nom::character::complete::{char, line_ending, multispace1, not_line_ending};

use nom::combinator::{eof, map_res, value};
use nom::error::{ErrorKind, ParseError};

use crate::slang::Span;
use nom::sequence::delimited;
use nom::{
    bytes::complete::tag, character::complete::multispace0, IResult, InputIter, InputLength,
    InputTake,
};

#[derive(Debug)]
pub struct Token<'a> {
    pub key: Span<'a>,
    pub value: Span<'a>,
}

/// Return the input slice up to the first occurrence of the parser,
/// and the result of the parser on match.
/// If the parser never matches, returns an error with code `ManyTill`
pub fn take_up_to<Input, Output, Error: ParseError<Input>, P>(
    mut parser: P,
) -> impl FnMut(Input) -> IResult<Input, (Input, Output), Error>
where
    P: FnMut(Input) -> IResult<Input, Output, Error>,
    Input: InputLength + InputIter + InputTake,
{
    move |i: Input| {
        let input = i;
        for (index, _) in input.iter_indices() {
            let (rest, front) = input.take_split(index);
            match parser(rest) {
                Ok((remainder, output)) => return Ok((remainder, (front, output))),
                Err(_) => continue,
            }
        }
        Err(nom::Err::Error(Error::from_error_kind(
            input,
            ErrorKind::ManyTill,
        )))
    }
}

fn parse_assignment(input: Span) -> IResult<Span, ()> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = multispace0(input)?;
    Ok((input, ()))
}

fn extract_from_quotes(input: Span) -> IResult<Span, Span> {
    let (input, between) = delimited(char('"'), is_not("\""), char('"'))(input)?;
    let (input, _) = whitespace(input)?;
    let (input, _) = eof(input)?;
    Ok((input, between))
}

fn multiline_comment(i: Span) -> IResult<Span, Span> {
    delimited(tag("/*"), take_until("*/"), tag("*/"))(i)
}

fn single_comment(i: Span) -> IResult<Span, Span> {
    delimited(
        alt((tag("//"), tag("#"))),
        not_line_ending,
        alt((line_ending, eof)),
    )(i)
}

fn whitespace(i: Span) -> IResult<Span, ()> {
    value(
        (), // Output is thrown away.
        multispace0,
    )(i)
}

fn optional_quotes(input: Span) -> IResult<(), Span> {
    let input = if let Ok((_, between)) = extract_from_quotes(input) {
        between
    } else {
        input
    };
    Ok(((), input))
}

fn parse_reference(input: Span) -> IResult<Span, Token> {
    let (input, key) = tag("#reference")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, (_, value)) = map_res(not_line_ending, optional_quotes)(input)?;
    Ok((input, Token { key, value }))
}
fn parse_key_value(input: Span) -> IResult<Span, Token> {
    let (input, (key, _)) = take_up_to(parse_assignment)(input)?;
    let (input, (_, value)) = map_res(not_line_ending, optional_quotes)(input)?;
    let (_, value) =
        take_until::<_, _, nom::error::Error<Span>>("//")(value).unwrap_or((input, value));
    let (_, value) =
        take_until::<_, _, nom::error::Error<Span>>("#")(value).unwrap_or((input, value));
    let (_, (_, value)) = map_res(not_line_ending, optional_quotes)(value)?;
    Ok((input, Token { key, value }))
}

fn parse_tokens(mut span: Span) -> IResult<Span, Vec<Token>> {
    let mut values = Vec::new();
    while !span.is_empty() {
        // important to munch whitespace first.
        if let Ok((input, _)) = whitespace(span) {
            span = input;
        }
        // handle references before comments because comments can start with #
        if let Ok((input, token)) = parse_reference(span) {
            span = input;
            values.push(token);
            continue;
        }
        if let Ok((input, _)) = multiline_comment(span) {
            span = input;
            continue;
        }
        if let Ok((input, _)) = single_comment(span) {
            span = input;
            continue;
        }
        let (input, token) = parse_key_value(span)?;
        span = input;
        values.push(token)
    }
    Ok((span, values))
}

pub fn do_lex(input: &str) -> Result<Vec<Token>, ParsePresetError> {
    let span = Span::new(input.trim_end());
    let (_, tokens) = parse_tokens(span).map_err(|e| match e {
        nom::Err::Error(e) | nom::Err::Failure(e) => {
            let input: Span = e.input;
            println!("{:?}", input);
            ParsePresetError::LexerError {
                offset: input.location_offset(),
                row: input.location_line(),
                col: input.get_column(),
            }
        }
        _ => ParsePresetError::LexerError {
            offset: 0,
            row: 0,
            col: 0,
        },
    })?;
    Ok(tokens)
}

#[cfg(test)]
mod test {
    use crate::slang::token::{do_lex, single_comment};

    #[test]
    fn parses_single_line_comment() {
        let parsed =
            single_comment("// Define textures to be used by the different passes\ntetx=n".into());
        eprintln!("{parsed:?}")
    }

    #[test]
    fn parses_key_value_line() {
        let parsed = do_lex(TEST3);
        eprintln!("{parsed:#?}")
    }

    // todo: fix
    const TEST2: &str = r#"
// Color Correction with Dogway's awesome Grade shader
// Grade is after Afterglow so that brightening the black level does not break the afterglow
shader9 = ../../shaders/dogway/hsm-grade.slang
    "#;
    const TEST: &str = r#"
#reference "../../alt"
shaders = 54

shader0 = ../../shaders/base/add-params-all.slang
alias0 = "CorePass" # hello

shader1 = ../../shaders/hyllian/cubic/hsm-drez-b-spline-x.slang
filter_linear1 = false
scale_type_x1 = absolute
scale_x1 = 640
scale_type_y1 = viewport
scaley0 = 1.0
wrap_mode1 = "clamp_to_edge"

shader2 = ../../shaders/hyllian/cubic/hsm-drez-b-spline-y.slang
filter_linear2 = false
scale_type2 = absolute
scale_x2 = 640
scale_y2 = 480
wrap_mode2 = "clamp_to_edge"
alias2 = "DerezedPass"

shader3 = ../../shaders/base/add-negative-crop-area.slang
filter_linear3 = false
mipmap_input3 = false
srgb_framebuffer3 = true
scale_type3 = source
scale_x3 = 1
scale_y3 = 1
alias3 = "NegativeCropAddedPass"

shader4 = ../../shaders/base/cache-info-all-params.slang
filter_linear4 = false
scale_type4 = source
scale4 = 1.0
alias4 = "InfoCachePass"

shader5 = ../../shaders/base/text-std.slang
filter_linear5 = false
float_framebuffer5 = true
scale_type5 = source
scale5 = 1.0
alias5 = "TextPass"

shader6 = ../../shaders/base/intro.slang
filter_linear6 = false
float_framebuffer6 = true
scale_type6 = source
scale6 = 1.0
alias6 = "IntroPass"

shader7 = ../../shaders/dedither/dedither-gamma-prep-1-before.slang
alias7 = LinearGamma

shader8 = ../../shaders/hyllian/checkerboard-dedither/checkerboard-dedither-pass1.slang
shader9 = ../../shaders/hyllian/checkerboard-dedither/checkerboard-dedither-pass2.slang
shader10 = ../../shaders/hyllian/checkerboard-dedither/checkerboard-dedither-pass3.slang
alias10 = "PreMdaptPass"

// De-Dithering - Mdapt
shader11 = ../../shaders/mdapt/hsm-mdapt-pass0.slang
shader12 = ../../shaders/mdapt/hsm-mdapt-pass1.slang
shader13 = ../../shaders/mdapt/hsm-mdapt-pass2.slang
shader14 = ../../shaders/mdapt/hsm-mdapt-pass3.slang
shader15 = ../../shaders/mdapt/hsm-mdapt-pass4.slang

shader16 = ../../shaders/dedither/dedither-gamma-prep-2-after.slang

shader17 = ../../shaders/ps1dither/hsm-PS1-Undither-BoxBlur.slang

shader18 = ../../shaders/guest/extras/hsm-sharpsmoother.slang

shader19 = ../../shaders/base/stock.slang
alias19 = refpass

shader20 = ../../shaders/scalefx/hsm-scalefx-pass0.slang
filter_linear20 = false
scale_type20 = source
scale20 = 1.0
float_framebuffer20 = true
alias20 = scalefx_pass0

shader21 = ../../shaders/scalefx/hsm-scalefx-pass1.slang
filter_linear21 = false
scale_type21 = source
scale21 = 1.0
float_framebuffer12 = true

shader22 = ../../shaders/scalefx/hsm-scalefx-pass2.slang
filter_linear22 = false
scale_type22 = source
scale22 = 1.0

shader23 = ../../shaders/scalefx/hsm-scalefx-pass3.slang
filter_linear23 = false
scale_type23 = source
scale23 = 1.0

shader24 = ../../shaders/scalefx/hsm-scalefx-pass4.slang
filter_linear24 = false
scale_type24 = source
scale24 = 3

shader25 = ../../shaders/base/stock.slang
alias25 = "PreCRTPass"

shader26 = ../../shaders/guest/hsm-afterglow0.slang
filter_linear26 = true
scale_type26 = source
scale26 = 1.0
alias26 = "AfterglowPass"

shader27 = ../../shaders/guest/hsm-pre-shaders-afterglow.slang
filter_linear27 = true
scale_type27 = source
mipmap_input27 = true
scale27 = 1.0

// Color Correction with Dogway's awesome Grade shader
// Grade is after Afterglow so that brightening the black level does not break the afterglow
shader28 = ../../shaders/dogway/hsm-grade.slang
filter_linear28 = true
scale_type28 = source
scale28 = 1.0

shader29 = ../../shaders/base/stock.slang
alias29 = "PrePass0"

shader30 = ../../shaders/guest/ntsc/hsm-ntsc-pass1.slang
filter_linear30 = false
float_framebuffer30 = true
scale_type_x30 = source
scale_type_y30 = source
scale_x30 = 4.0
scale_y30 = 1.0
frame_count_mod30 = 2
alias30 = NPass1

shader31 = ../../shaders/guest/ntsc/hsm-ntsc-pass2.slang
float_framebuffer31 = true
filter_linear31 = true
scale_type31 = source
scale_x31 = 0.5
scale_y31 = 1.0

shader32 = ../../shaders/guest/ntsc/hsm-ntsc-pass3.slang
filter_linear32 = true
scale_type32 = source
scale_x32 = 1.0
scale_y32 = 1.0

shader33 =  ../../shaders/guest/hsm-custom-fast-sharpen.slang
filter_linear33 = true
scale_type33 = source
scale_x33 = 1.0
scale_y33 = 1.0

shader34 = ../../shaders/base/stock.slang
filter_linear34 = true
scale_type34 = source
scale_x34 = 1.0
scale_y34 = 1.0
alias34 = "PrePass"
mipmap_input34 = true

shader35 = ../../shaders/guest/hsm-avg-lum.slang
filter_linear35 = true
scale_type35 = source
scale35 = 1.0
mipmap_input35 = true
alias35 = "AvgLumPass"

// Pass referenced by subsequent blurring passes and crt pass
shader36 = ../../shaders/guest/hsm-interlace-and-linearize.slang
filter_linear36 = true
scale_type36 = source
scale36 = 1.0
float_framebuffer36 = true
alias36 = "LinearizePass"

shader37 = ../../shaders/guest/hsm-crt-guest-advanced-ntsc-pass1.slang
filter_linear37 = true
scale_type_x37 = viewport
scale_x37 = 1.0
scale_type_y37 = source
scale_y37 = 1.0
float_framebuffer37 = true
alias37 = Pass1

shader38 = ../../shaders/guest/hsm-gaussian_horizontal.slang
filter_linear38 = true
scale_type_x38 = absolute
scale_x38 = 640.0
scale_type_y38 = source
scale_y38 = 1.0
float_framebuffer38 = true

shader39 = ../../shaders/guest/hsm-gaussian_vertical.slang
filter_linear39 = true
scale_type_x39 = absolute
scale_x39 = 640.0
scale_type_y39 = absolute
scale_y39 = 480.0
float_framebuffer39 = true
alias39 = GlowPass

shader40 = ../../shaders/guest/hsm-bloom_horizontal.slang
filter_linear40 = true
scale_type_x40 = absolute
scale_x40 = 640.0
scale_type_y40 = absolute
scale_y40 = 480.0
float_framebuffer40 = true

shader41 = ../../shaders/guest/hsm-bloom_vertical.slang
filter_linear41 = true
scale_type_x41 = absolute
scale_x41 = 640.0
scale_type_y41 = absolute
scale_y41 = 480.0
float_framebuffer41 = true
alias41 = BloomPass

shader42 = ../../shaders/guest/hsm-crt-guest-advanced-ntsc-pass2.slang
filter_linear42 = true
float_framebuffer42 = true
scale_type42 = viewport
scale_x42 = 1.0
scale_y42 = 1.0

shader43 = ../../shaders/guest/hsm-deconvergence.slang
filter_linear43 = true
scale_type43 = viewport
scale_x43 = 1.0
scale_y43 = 1.0

shader44 = ../../shaders/base/post-crt-prep-image-layers.slang
alias44 = "MBZ_PostCRTPass"

// Reduce Resolution  ----------------------------------------------------------------
//      Reduce the resolution to a small static size regardless of final resolution
//      Allows consistent look and faster at different final resolutions for blur
//      Mipmap option allows downscaling without artifacts
shader45 = ../../shaders/base/linearize-crt.slang
mipmap_input45 = true
filter_linear45 = true
scale_type45 = absolute
// scale_x45 = 480
// scale_y45 = 270
// scale_x45 = 960
// scale_y45 = 540
scale_x45 = 800
scale_y45 = 600
alias45 = "BR_MirrorLowResPass"

// Add Blur for the Reflection (Horizontal) ----------------------------------------------------------------
shader46 = ../../shaders/base/blur-outside-screen-horiz.slang
mipmap_input46 = true
filter_linear46 = true

// Add Blur for the Reflection (Vertical) ----------------------------------------------------------------
shader47 = ../../shaders/base/blur-outside-screen-vert.slang
filter_linear47 = true
alias47 = "BR_MirrorBlurredPass"

// Reduce resolution ----------------------------------------------------------------
// Reduced to a very small amount so we can create a blur which will create a glow from the screen
//      Mipmap option allows smoother downscaling
shader48 = ../../../../blurs/shaders/royale/blur9x9.slang
mipmap_input48 = true
filter_linear48 = true
scale_type48 = absolute
scale_x48 = 128
scale_y48 = 128
alias48 = "BR_MirrorReflectionDiffusedPass"

// Add Diffused glow all around the screen ----------------------------------------------------------------
//      Blurred so much that it's non directional
//      Mipmap option allows downscaling without artifacts
shader49 = ../../../../blurs/shaders/royale/blur9x9.slang
mipmap_input49 = true
filter_linear49 = true
scale_type49 = absolute
scale_x49 = 12
scale_y49 = 12
alias49 = "BR_MirrorFullscreenGlowPass"

// Bezel Reflection ----------------------------------------------------------------
shader50 = ../../shaders/base/reflection.slang
scale_type50 = viewport
float_framebuffer50 = true
alias50 = "BR_CRTAndReflectionPass"

// Bezel Generation & Composite of Image Layers ----------------------------------------------------------------

shader51 = ../../shaders/base/bezel-images-under-crt.slang
filter_linear51 = true
scale_type51 = viewport
float_framebuffer51 = true
alias51 = "BR_LayersUnderCRTPass"

shader52 = ../../shaders/base/bezel-images-over-crt.slang
filter_linear52 = true
scale_type52 = viewport
float_framebuffer52 = true
alias52 = "BR_LayersOverCRTPass"

// Combine Passes ----------------------------------------------------------------
shader53 = ../../shaders/base/combine-passes.slang
scale_type53 = viewport
alias53 = "CombinePass"
// Define textures to be used by the different passes
textures = "SamplerLUT1;SamplerLUT2;SamplerLUT3;SamplerLUT4;IntroImage;ScreenPlacementImage;TubeDiffuseImage;TubeColoredGelImage;TubeShadowImage;TubeStaticReflectionImage;BackgroundImage;BackgroundVertImage;ReflectionMaskImage;FrameTextureImage;CabinetGlassImage;DeviceImage;DeviceVertImage;DeviceLEDImage;DecalImage;NightLightingImage;NightLighting2Image;LEDImage;TopLayerImage;"

SamplerLUT1 = ../../shaders/guest/lut/trinitron-lut.png
SamplerLUT1_linear = true
SamplerLUT2 = ../../shaders/guest/lut/inv-trinitron-lut.png
SamplerLUT2_linear = true
SamplerLUT3 = ../../shaders/guest/lut/nec-lut.png
SamplerLUT3_linear = true
SamplerLUT4 = ../../shaders/guest/lut/ntsc-lut.png
SamplerLUT4_linear = true

IntroImage = ../../shaders/textures/IntroImage_MegaBezelLogo.png
IntroImage_linear = true
IntroImage_mipmap = 1

ScreenPlacementImage = ../../shaders/textures/Placeholder_Transparent_16x16.png
ScreenPlacementImage_linear = false

TubeDiffuseImage = ../../shaders/textures/Tube_Diffuse_2390x1792.png
TubeDiffuseImage_linear = true
TubeDiffuseImage_mipmap = 1

TubeColoredGelImage = ../../shaders/textures/Colored_Gel_Rainbow.png
TubeColoredGelImage_linear = true
TubeColoredGelImage_mipmap = 1

TubeShadowImage = ../../shaders/textures/Tube_Shadow_1600x1200.png
TubeShadowImage_linear = true
TubeShadowImage_mipmap = 1

TubeStaticReflectionImage = ../../shaders/textures/TubeGlassOverlayImageCropped_1440x1080.png
TubeStaticReflectionImage_linear = true
TubeStaticReflectionImage_mipmap = 1

ReflectionMaskImage = ../../shaders/textures/Placeholder_White_16x16.png
ReflectionMaskImage_linear = true
ReflectionMaskImage_mipmap = 1

FrameTextureImage = ../../shaders/textures/FrameTexture_2800x2120.png
FrameTextureImage_linear = true
FrameTextureImage_mipmap = 1

BackgroundImage = ../../shaders/textures/BackgroundImage_Carbon_3840x2160.png
BackgroundImage_linear = true
BackgroundImage_mipmap = 1

BackgroundVertImage = ../../shaders/textures/Placeholder_Transparent_16x16.png
BackgroundVertImage_linear = true
BackgroundVertImage_mipmap = 1

CabinetGlassImage = ../../shaders/textures/Placeholder_Transparent_16x16.png
CabinetGlassImage_linear = true
CabinetGlassImage_mipmap = 1

DeviceImage = ../../shaders/textures/Placeholder_Transparent_16x16.png
DeviceImage_linear = true
DeviceImage_mipmap = 1

DeviceVertImage = ../../shaders/textures/Placeholder_Transparent_16x16.png
DeviceVertImage_linear = true
DeviceVertImage_mipmap = 1

DeviceLEDImage = ../../shaders/textures/Placeholder_Transparent_16x16.png
DeviceLEDImage_linear = true
DeviceLEDImage_mipmap = 1

DecalImage = ../../shaders/textures/Placeholder_Transparent_16x16.png
DecalImage_linear = true
DecalImage_mipmap = 1

NightLightingImage = ../../shaders/textures/NightLightingClose_1920x1080.png
NightLightingImage_linear = true
NightLightingImage_mipmap = 1

NightLighting2Image = ../../shaders/textures/NightLightingFar_1920x1080.png
NightLighting2Image_linear = true
NightLighting2Image_mipmap = 1

LEDImage = ../../shaders/textures/Placeholder_Transparent_16x16.png
LEDImage_linear = true
LEDImage_mipmap = 1

TopLayerImage = ../../shaders/textures/Placeholder_Transparent_16x16.png
TopLayerImage_linear = true
TopLayerImage_mipmap = 1

// Use for matching vanilla GDV-Advanced
// HSM_ASPECT_RATIO_MODE = 6
// HSM_CURVATURE_MODE = 0

// SMOOTH-ADV
HSM_DEDITHER_MODE = 1

HSM_SCALEFX_ON = 1

HSM_CORE_RES_SAMPLING_MULT_SCANLINE_DIR = 300
HSM_CORE_RES_SAMPLING_MULT_OPPOSITE_DIR = 125
HSM_DOWNSAMPLE_BLUR_SCANLINE_DIR = 0
HSM_DOWNSAMPLE_BLUR_OPPOSITE_DIR = 0

ntsc_scale = 0.4

shadowMask = 3

// NTSC Parameters
GAMMA_INPUT = 2.0
gamma_out = 1.95

// DREZ Parameters
SHARPEN = 0"#;

    const TEST3: &str = r#"// DUIMON MEGA BEZEL GRAPHICS AND PRESETS | https://duimon.github.io/Gallery-Guides/ | duimonmb@gmail.com
// SOME RIGHTS RESERVED - RELEASED UNDER CC BY NC ND LICENSE https://creativecommons.org/licenses/by-nc-nd/4.0/deed
// ----------------------------------------------------------------------------------------------------------------

// PRESET START
// ----------------------------------------------------------------------------------------------------------------

// SHADER :: CONNECTOR | Interface to Mega Bezel Presets folders.
// Edit the target file in the following reference to globally define the base preset.
// ----------------------------------------------------------------------------------------------------------------

#reference "../../../zzz_global_params/Base_Shader/ADV_Bezel.slangp"

// SHADER :: CONNECTOR :: LOCAL OVERRIDES | Interface to specific base presets.
// Comment out the top reference line and uncomment the following reference line to locally define the base preset.
// Keep in mind that some of the base presets use Integer Scale and may yield unexpected results. (e.g. Megatron)

//#reference "../../../zzz_global_params/Local_Shader/ADV_06.slangp"

// "ADV_06" matches the default "MBZ__1__ADV__GDV.slangp".
// Replace the "06" with any from the following list.
// 01. SMOOTH-ADV__GDV                  08. ADV__GDV-MINI-NTSC
// 02. SMOOTH-ADV__GDV-NTSC             09. ADV__GDV-NTSC
// 03. SMOOTH-ADV__MEGATRON             10. ADV__MEGATRON
// 04. SMOOTH-ADV__MEGATRON-NTSC        11. ADV__MEGATRON-NTSC
// 05. ADV__EASYMODE                    12. ADV-RESHADE-FX__GDV
// 06. ADV__GDV                         13. ADV-SUPER-XBR__GDV
// 07. ADV__GDV-MINI                    14. ADV-SUPER-XBR__GDV-NTSC

// INTRO | Intro animation
// ----------------------------------------------------------------------------------------------------------------

// ON
#reference "../../../zzz_global_params/Intro/on.params"
// ON - No Image
//#reference "../../../zzz_global_params/Intro/on_no_image.params"
// ON - Default Mega Bezel intro
//#reference "../../../zzz_global_params/Intro/on_default.params"
// OFF
//#reference "../../../zzz_global_params/Intro/off.params"

// DEVICE | Screen/Monitor/CRT/TV settings
// ----------------------------------------------------------------------------------------------------------------

// DEVICE :: BASE
#reference "../../../res/bezel/Nintendo_GBA/bezel.params"

// DEVICE :: SCALING
#reference "../../../res/scale/Nintendo_GBA/bezel.params"

// DEVICE :: CRT
#reference "../../../res/crt/Nintendo_GBA/bezel.params"

// IMAGE LAYERS
// ----------------------------------------------------------------------------------------------------------------
#reference "../../../res/layers/Nintendo_GBA/bezel.params"

// HSV :: Hue, Saturation, and Value parameters
// ----------------------------------------------------------------------------------------------------------------

// GRAPHICS OVERRIDES | Overrides for Image layers, scaling, etc
// that are not related to Guest's shader. (Three examples are provided)
// These are intended for [Bezel] versions and the following reference should be left commented out for others.
// ----------------------------------------------------------------------------------------------------------------

// GRAPHICS :: OVERRIDES
//#reference "../../../res/overrides/batocera.params"
//#reference "../../../res/overrides/batocera_nocurve.params"
//#reference "../../../res/overrides/batocera_hud.params"

// GLOBAL GRAPHICS :: OVERRIDES
// The user can edit the "user.params" to globally change the presets.
// These are for the bezel, frame, and other graphic attributes.
// Examples are included in the params file and commented out.
// These are also intended for [Bezel] versions and the following reference should be left commented out for others.
#reference "../../../zzz_global_params/Graphics/user.params"

// The following is restricted to the [Custom-Bezel_002] presets.
// One example is included in the params file and commented out.
//#reference "../../../zzz_global_params/Graphics/user2.params"

// SHADER OVERRIDES | Place *.params references to Guest derivatives here.
// (Make sure you are using ADV__GDV, STD__GDV, or POTATO__GDV base presets for variations on the Guest shader.)
// Two examples were kindly provided by guest.r. ;-)
// ----------------------------------------------------------------------------------------------------------------

// SHADER :: OVERRIDES
//#reference "../../../res/overrides_shader/guest_custom_aperture.params"
//#reference "../../../res/overrides_shader/guest_custom_slotmask.params"

// GLOBAL SHADER :: OVERRIDES
// The user can edit the target params file to globally change the presets.
// To use community params that require another base preset, change the global base reference to match.
// Examples are included in the params file and commented out.
// Separate folders let users change global settings on each of the sets.
// These are intentionally commented out for LCD-GRID presets.
//#reference "../../../zzz_global_params/Shader/ADV/user_Bezel.params"
//#reference "../../../zzz_global_params/Shader/ADV_DREZ/user_Bezel.params"
//#reference "../../../zzz_global_params/Shader/STD/user_Bezel.params"
//#reference "../../../zzz_global_params/Shader/STD_DREZ/user_Bezel.params"
//#reference "../../../zzz_global_params/Shader/LITE/user_Bezel.params"

// AMBIENT LIGHTING
//#reference "../../../res/lighting/night.params"

// PRESET END
// ----------------------------------------------------------------------------------------------------------------
"#;
}
