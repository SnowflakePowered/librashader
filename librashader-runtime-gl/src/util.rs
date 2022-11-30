use gl::types::{GLenum, GLint, GLuint};
use librashader_common::Size;
use librashader_reflect::back::cross::GlVersion;
use librashader_reflect::reflect::semantics::BindingStage;
use librashader_runtime::uniforms::{BindUniform, UniformStorage, UniformScalar};
use crate::binding::UniformLocation;

pub trait RingBuffer<T> {
    fn current(&self) -> &T;
    fn current_mut(&mut self) -> &mut T;
    fn next(&mut self);
}

impl<T, const SIZE: usize> RingBuffer<T> for InlineRingBuffer<T, SIZE> {
    fn current(&self) -> &T {
        &self.items[self.index]
    }

    fn current_mut(&mut self) -> &mut T {
        &mut self.items[self.index]
    }

    fn next(&mut self) {
        self.index += 1;
        if self.index >= SIZE {
            self.index = 0
        }
    }
}

pub struct InlineRingBuffer<T, const SIZE: usize> {
    items: [T; SIZE],
    index: usize,
}

impl<T, const SIZE: usize> InlineRingBuffer<T, SIZE>
where
    T: Copy,
    T: Default,
{
    pub fn new() -> Self {
        Self {
            items: [T::default(); SIZE],
            index: 0,
        }
    }

    pub fn items(&self) -> &[T; SIZE] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut [T; SIZE] {
        &mut self.items
    }
}

pub unsafe fn gl_compile_shader(stage: GLenum, source: &str) -> GLuint {
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

    if compile_status == 0 {
        panic!("failed to compile")
    }
    shader
}

pub fn gl_get_version() -> GlVersion {
    let mut maj_ver = 0;
    let mut min_ver = 0;
    unsafe {
        gl::GetIntegerv(gl::MAJOR_VERSION, &mut maj_ver);
        gl::GetIntegerv(gl::MINOR_VERSION, &mut min_ver);
    }

    match maj_ver {
        3 => match min_ver {
            3 => GlVersion::V3_30,
            2 => GlVersion::V1_50,
            1 => GlVersion::V1_40,
            0 => GlVersion::V1_30,
            _ => GlVersion::V1_50,
        }
        4 => match min_ver {
            6 => GlVersion::V4_60,
            5 => GlVersion::V4_50,
            4 => GlVersion::V4_40,
            3 => GlVersion::V4_30,
            2 => GlVersion::V4_20,
            1 => GlVersion::V4_10,
            0 => GlVersion::V4_00,
            _ => GlVersion::V1_50
        }
        _ => GlVersion::V1_50
    }

}

pub fn gl_u16_to_version(version: u16) -> GlVersion {
    match version {
        300 => GlVersion::V1_30,
        310 => GlVersion::V1_40,
        320 => GlVersion::V1_50,
        330 => GlVersion::V3_30,
        400 => GlVersion::V4_00,
        410 => GlVersion::V4_10,
        420 => GlVersion::V4_20,
        430 => GlVersion::V4_30,
        440 => GlVersion::V4_40,
        450 => GlVersion::V4_50,
        460 => GlVersion::V4_60,
        _ => GlVersion::V1_50
    }
}
