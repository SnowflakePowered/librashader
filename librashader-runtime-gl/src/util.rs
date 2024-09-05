use gl::types::{GLenum, GLuint};

use crate::error;
use crate::error::FilterChainError;
use librashader_reflect::back::glsl::GlslVersion;

pub unsafe fn gl_compile_shader(stage: GLenum, source: &str) -> error::Result<GLuint> {
    let (shader, compile_status) = unsafe {
        let shader = gl::CreateShader(stage);
        gl::ShaderSource(
            shader,
            1,
            &source.as_bytes().as_ptr().cast(),
            std::ptr::null(),
        );
        gl::CompileShader(shader);
        let mut compile_status = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut compile_status);
        (shader, compile_status)
    };

    if compile_status == 0 {
        Err(FilterChainError::GlCompileError)
    } else {
        Ok(shader)
    }
}

pub fn gl_get_version() -> GlslVersion {
    let mut maj_ver = 0;
    let mut min_ver = 0;
    unsafe {
        gl::GetIntegerv(gl::MAJOR_VERSION, &mut maj_ver);
        gl::GetIntegerv(gl::MINOR_VERSION, &mut min_ver);
    }

    match maj_ver {
        3 => match min_ver {
            3 => GlslVersion::Glsl330,
            2 => GlslVersion::Glsl150,
            1 => GlslVersion::Glsl140,
            0 => GlslVersion::Glsl130,
            _ => GlslVersion::Glsl150,
        },
        4 => match min_ver {
            6 => GlslVersion::Glsl460,
            5 => GlslVersion::Glsl450,
            4 => GlslVersion::Glsl440,
            3 => GlslVersion::Glsl430,
            2 => GlslVersion::Glsl420,
            1 => GlslVersion::Glsl410,
            0 => GlslVersion::Glsl400,
            _ => GlslVersion::Glsl150,
        },
        _ => GlslVersion::Glsl150,
    }
}

pub fn gl_u16_to_version(version: u16) -> GlslVersion {
    match version {
        0 => gl_get_version(),
        300 => GlslVersion::Glsl130,
        310 => GlslVersion::Glsl140,
        320 => GlslVersion::Glsl150,
        330 => GlslVersion::Glsl330,
        400 => GlslVersion::Glsl400,
        410 => GlslVersion::Glsl410,
        420 => GlslVersion::Glsl420,
        430 => GlslVersion::Glsl430,
        440 => GlslVersion::Glsl440,
        450 => GlslVersion::Glsl450,
        460 => GlslVersion::Glsl460,
        _ => GlslVersion::Glsl150,
    }
}
