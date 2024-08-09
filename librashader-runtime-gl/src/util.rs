use glow::HasContext;

use crate::error;
use crate::error::FilterChainError;
use librashader_reflect::back::glsl::GlslVersion;

pub fn gl_compile_shader(
    context: &glow::Context,
    stage: u32,
    source: &str,
) -> error::Result<glow::Shader> {
    unsafe {
        let shader = context
            .create_shader(stage)
            .map_err(|_| FilterChainError::GlCompileError)?;

        context.shader_source(shader, &source);
        context.compile_shader(shader);
        let compile_status = context.get_shader_compile_status(shader);

        if !compile_status {
            Err(FilterChainError::GlCompileError)
        } else {
            Ok(shader)
        }
    }
}

pub fn gl_get_version(context: &glow::Context) -> GlslVersion {
    let version = context.version();

    let maj_ver = version.major;
    let min_ver = version.minor;

    match maj_ver {
        3 => match min_ver {
            3 => GlslVersion::V3_30,
            2 => GlslVersion::V1_50,
            1 => GlslVersion::V1_40,
            0 => GlslVersion::V1_30,
            _ => GlslVersion::V1_50,
        },
        4 => match min_ver {
            6 => GlslVersion::V4_60,
            5 => GlslVersion::V4_50,
            4 => GlslVersion::V4_40,
            3 => GlslVersion::V4_30,
            2 => GlslVersion::V4_20,
            1 => GlslVersion::V4_10,
            0 => GlslVersion::V4_00,
            _ => GlslVersion::V1_50,
        },
        _ => GlslVersion::V1_50,
    }
}

pub fn gl_u16_to_version(context: &glow::Context, version: u16) -> GlslVersion {
    match version {
        0 => gl_get_version(context),
        300 => GlslVersion::V1_30,
        310 => GlslVersion::V1_40,
        320 => GlslVersion::V1_50,
        330 => GlslVersion::V3_30,
        400 => GlslVersion::V4_00,
        410 => GlslVersion::V4_10,
        420 => GlslVersion::V4_20,
        430 => GlslVersion::V4_30,
        440 => GlslVersion::V4_40,
        450 => GlslVersion::V4_50,
        460 => GlslVersion::V4_60,
        _ => GlslVersion::V1_50,
    }
}
