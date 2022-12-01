use gl::types::{GLenum, GLuint};

use librashader_reflect::back::cross::GlslVersion;

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

pub fn gl_get_version() -> GlslVersion {
    let mut maj_ver = 0;
    let mut min_ver = 0;
    unsafe {
        gl::GetIntegerv(gl::MAJOR_VERSION, &mut maj_ver);
        gl::GetIntegerv(gl::MINOR_VERSION, &mut min_ver);
    }

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

pub fn gl_u16_to_version(version: u16) -> GlslVersion {
    match version {
        0 => gl_get_version(),
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
