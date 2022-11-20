use gl::types::{GLenum, GLsizei, GLuint};
use librashader::{FilterMode, ShaderFormat, WrapMode};
use librashader_presets::{Scale2D, ScaleType, Scaling};
use crate::util;
use crate::util::{GlImage, Size, Texture, Viewport};

#[derive(Debug)]
pub struct Framebuffer {
    pub image: GLuint,
    pub size: Size,
    pub format: GLenum,
    pub max_levels: u32,
    pub levels: u32,
    pub framebuffer: GLuint,
    pub init: bool
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
            size: Size { width: 1, height: 1 },
            format: 0,
            max_levels,
            levels: 0,
            framebuffer,
            init: false
        }
    }

    pub fn new_from_raw(texture: GLuint, handle: GLuint, format: GLenum, size: Size, miplevels: u32) -> Framebuffer {
        Framebuffer {
            image: texture,
            size,
            format,
            max_levels: miplevels,
            levels: miplevels,
            framebuffer: handle,
            init: true
        }
    }

    pub fn as_texture(&self, filter: FilterMode, wrap_mode: WrapMode) -> Texture {
        Texture {
            image: GlImage {
                handle: self.image,
                format: self.format,
                size: self.size,
                padded_size: Default::default()
            },
            filter,
            mip_filter: filter,
            wrap_mode
        }
    }

    pub fn scale(&mut self, scaling: Scale2D, format: ShaderFormat, viewport: &Viewport, original: &Texture, source: &Texture) -> Size {
        let mut width = 0f32;
        let mut height = 0f32;

        match scaling.x {
            Scaling {
                scale_type: ScaleType::Input,
                factor
            } => {
                width = source.image.size.width * factor
            },
            Scaling {
                scale_type: ScaleType::Absolute,
                factor
            } => {
                width = factor.into()
            }
            Scaling {
                scale_type: ScaleType::Viewport,
                factor
            } => {
                width = viewport.output.size.width * factor
            }
        };

        match scaling.y {
            Scaling {
                scale_type: ScaleType::Input,
                factor
            } => {
                height = source.image.size.height * factor
            },
            Scaling {
                scale_type: ScaleType::Absolute,
                factor
            } => {
                height = factor.into()
            }
            Scaling {
                scale_type: ScaleType::Viewport,
                factor
            } => {
                height = viewport.output.size.height * factor
            }
        };

        let size = Size {
            width: width.round() as u32,
            height: height.round() as u32
        };

        if self.size != size {
            self.size = size;

            self.init(size,if format == ShaderFormat::Unknown {
                ShaderFormat::R8G8B8A8Unorm
            } else {
                format
            });
        }
        size
    }

    fn init(&mut self, mut size: Size, mut format: impl Into<GLenum>) {
        self.format = format.into();
        self.size = size;

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer);

            // reset the framebuffer image
            if self.image != 0 {
                gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, 0, 0);
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

            gl::TexStorage2D(gl::TEXTURE_2D, self.levels as GLsizei, self.format, size.width as GLsizei, size.height as GLsizei);
            gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                   gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, self.image, 0);

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                match status {
                    gl::FRAMEBUFFER_UNSUPPORTED => {
                        eprintln!("unsupported fbo");

                        gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                                 gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, 0, 0);
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

                        gl::TexStorage2D(gl::TEXTURE_2D, self.levels as GLsizei, ShaderFormat::R8G8B8A8Unorm.into(), size.width as GLsizei, size.height as GLsizei);
                        gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                                 gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, self.image, 0);
                        self.init = gl::CheckFramebufferStatus(gl::FRAMEBUFFER) == gl::FRAMEBUFFER_COMPLETE;
                    }
                    _ => panic!("failed to complete: {status:x}")
                }
            } else {
                self.init = true;
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            if self.framebuffer != 0 {
                gl::DeleteFramebuffers(1, &self.framebuffer);
            }
            if self.image != 0 {
                gl::DeleteTextures(1, &self.image);
            }
        }
    }
}