use gl::types::{GLenum, GLint, GLuint};
use librashader::{FilterMode, WrapMode};

pub fn calc_miplevel(width: u32, height: u32) -> u32 {
    let mut size = std::cmp::max(width, height);
    let mut levels = 0;
    while size != 0 {
        levels += 1;
        size >>= 1;
    }

    return levels;
}

#[derive(Debug, Copy, Clone)]
pub struct Texture {
    pub image: GlImage,
    pub filter: FilterMode,
    pub mip_filter: FilterMode,
    pub wrap_mode: WrapMode
}

#[derive(Debug, Copy, Clone)]
pub struct Viewport {
    pub x: i32,
    pub y: i32,
    pub size: Size,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct GlImage {
    pub handle: GLuint,
    pub format: GLenum,
    pub size: Size,
    pub padded_size: Size
}

impl <T, const SIZE: usize> RingBuffer<T, SIZE> {
    pub fn current(&self) -> &T {
        &self.items[self.index]
    }

    pub fn next(&mut self) {
        self.index += 1;
        if self.index >= SIZE {
            self.index = 0
        }
    }
}

pub struct RingBuffer<T, const SIZE: usize> {
    items: [T; SIZE],
    index: usize
}

impl <T, const SIZE: usize> RingBuffer<T, SIZE>
where T: Copy, T: Default
{
    pub fn new() -> Self {
        Self {
            items: [T::default(); SIZE],
            index: 0
        }
    }

    pub fn items(&self) -> &[T; SIZE] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut [T; SIZE] {
        &mut self.items
    }
}
