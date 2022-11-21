use crate::util;
use crate::util::Texture;
use gl::types::{GLenum, GLint, GLsizei, GLuint};
use librashader_common::{FilterMode, ShaderFormat, Size, WrapMode};
use librashader_presets::{Scale2D, ScaleType, Scaling};

#[derive(Debug)]
pub struct Framebuffer {
    pub image: GLuint,
    pub handle: GLuint,
    pub size: Size<u32>,
    pub format: GLenum,
    pub max_levels: u32,
    pub levels: u32,
    is_raw: bool,
}

impl Framebuffer {
    pub fn new(max_levels: u32) -> Framebuffer {
        let mut framebuffer = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        Framebuffer {
            image: 0,
            size: Size {
                width: 1,
                height: 1,
            },
            format: 0,
            max_levels,
            levels: 0,
            handle: framebuffer,
            is_raw: false,
        }
    }

    pub fn new_from_raw(
        texture: GLuint,
        handle: GLuint,
        format: GLenum,
        size: Size<u32>,
        miplevels: u32,
    ) -> Framebuffer {
        Framebuffer {
            image: texture,
            size,
            format,
            max_levels: miplevels,
            levels: miplevels,
            handle,
            is_raw: true,
        }
    }

    pub(crate) fn as_texture(&self, filter: FilterMode, wrap_mode: WrapMode) -> Texture {
        Texture {
            image: GlImage {
                handle: self.image,
                format: self.format,
                size: self.size,
                padded_size: Default::default(),
            },
            filter,
            mip_filter: filter,
            wrap_mode,
        }
    }

    pub(crate) fn scale(
        &mut self,
        scaling: Scale2D,
        format: ShaderFormat,
        viewport: &Viewport,
        _original: &Texture,
        source: &Texture,
    ) -> Size<u32> {
        if self.is_raw {
            return self.size;
        }

        let mut width = 0f32;
        let mut height = 0f32;

        match scaling.x {
            Scaling {
                scale_type: ScaleType::Input,
                factor,
            } => width = source.image.size.width * factor,
            Scaling {
                scale_type: ScaleType::Absolute,
                factor,
            } => width = factor.into(),
            Scaling {
                scale_type: ScaleType::Viewport,
                factor,
            } => width = viewport.output.size.width * factor,
        };

        match scaling.y {
            Scaling {
                scale_type: ScaleType::Input,
                factor,
            } => height = source.image.size.height * factor,
            Scaling {
                scale_type: ScaleType::Absolute,
                factor,
            } => height = factor.into(),
            Scaling {
                scale_type: ScaleType::Viewport,
                factor,
            } => height = viewport.output.size.height * factor,
        };

        let size = Size {
            width: width.round() as u32,
            height: height.round() as u32,
        };

        if self.size != size {
            self.size = size;

            self.init(
                size,
                if format == ShaderFormat::Unknown {
                    ShaderFormat::R8G8B8A8Unorm
                } else {
                    format
                },
            );
        }
        size
    }

    pub(crate) fn clear(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.handle);
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    pub(crate) fn copy_from(&mut self, image: &GlImage) {
        if image.size != self.size || image.format != self.format {
            self.init(image.size, image.format);
        }

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.handle);

            gl::FramebufferTexture2D(
                gl::READ_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                image.handle,
                0,
            );

            gl::FramebufferTexture2D(
                gl::DRAW_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT1,
                gl::TEXTURE_2D,
                self.image,
                0,
            );
            gl::DrawBuffer(gl::COLOR_ATTACHMENT1);
            gl::BlitFramebuffer(
                0,
                0,
                self.size.width as GLint,
                self.size.height as GLint,
                0,
                0,
                self.size.width as GLint,
                self.size.height as GLint,
                gl::COLOR_BUFFER_BIT,
                gl::NEAREST,
            );

            // cleanup after ourselves.
            gl::FramebufferTexture2D(
                gl::READ_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                0,
                0,
            );

            gl::FramebufferTexture2D(
                gl::DRAW_FRAMEBUFFER,
                gl::COLOR_ATTACHMENT1,
                gl::TEXTURE_2D,
                0,
                0,
            );

            // set this back to color_attachment 0
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                self.image,
                0,
            );

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    // todo: fix panic
    pub(crate) fn init(&mut self, mut size: Size<u32>, format: impl Into<GLenum>) {
        if self.is_raw {
            return;
        }
        self.format = format.into();
        self.size = size;

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.handle);

            // reset the framebuffer image
            if self.image != 0 {
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    0,
                    0,
                );
                gl::DeleteTextures(1, &self.image);
            }

            gl::GenTextures(1, &mut self.image);
            gl::BindTexture(gl::TEXTURE_2D, self.image);

            if size.width == 0 {
                size.width = 1;
            }
            if size.height == 0 {
                size.height = 1;
            }

            self.levels = util::calc_miplevel(size.width, size.height);
            if self.levels > self.max_levels {
                self.levels = self.max_levels;
            }
            if self.levels == 0 {
                self.levels = 1;
            }

            gl::TexStorage2D(
                gl::TEXTURE_2D,
                self.levels as GLsizei,
                self.format,
                size.width as GLsizei,
                size.height as GLsizei,
            );
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                self.image,
                0,
            );

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                match status {
                    gl::FRAMEBUFFER_UNSUPPORTED => {
                        eprintln!("unsupported fbo");

                        gl::FramebufferTexture2D(
                            gl::FRAMEBUFFER,
                            gl::COLOR_ATTACHMENT0,
                            gl::TEXTURE_2D,
                            0,
                            0,
                        );
                        gl::DeleteTextures(1, &self.image);
                        gl::GenTextures(1, &mut self.image);
                        gl::BindTexture(1, self.image);

                        self.levels = util::calc_miplevel(size.width, size.height);
                        if self.levels > self.max_levels {
                            self.levels = self.max_levels;
                        }
                        if self.levels == 0 {
                            self.levels = 1;
                        }

                        gl::TexStorage2D(
                            gl::TEXTURE_2D,
                            self.levels as GLsizei,
                            ShaderFormat::R8G8B8A8Unorm.into(),
                            size.width as GLsizei,
                            size.height as GLsizei,
                        );
                        gl::FramebufferTexture2D(
                            gl::FRAMEBUFFER,
                            gl::COLOR_ATTACHMENT0,
                            gl::TEXTURE_2D,
                            self.image,
                            0,
                        );
                        // self.init =
                        //     gl::CheckFramebufferStatus(gl::FRAMEBUFFER) == gl::FRAMEBUFFER_COMPLETE;
                    }
                    _ => panic!("failed to complete: {status:x}"),
                }
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            if self.handle != 0 {
                gl::DeleteFramebuffers(1, &self.handle);
            }
            if self.image != 0 {
                gl::DeleteTextures(1, &self.image);
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Viewport<'a> {
    pub x: i32,
    pub y: i32,
    pub output: &'a Framebuffer,
    pub mvp: Option<&'a [f32]>,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct GlImage {
    pub handle: GLuint,
    pub format: GLenum,
    pub size: Size<u32>,
    pub padded_size: Size<u32>,
}
