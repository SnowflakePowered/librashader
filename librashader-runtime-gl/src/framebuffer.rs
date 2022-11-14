use gl::types::{GLenum, GLsizei, GLuint};
use librashader::ShaderFormat;
use crate::util;
use crate::util::Size;

pub struct Framebuffer {
    pub image: GLuint,
    pub size: Size,
    pub format: GLenum,
    pub max_levels: u32,
    pub levels: u32,
    pub framebuffer: GLuint,
    pub init: bool
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if self.framebuffer != 0 {
            unsafe {
                gl::DeleteFramebuffers(1, &self.framebuffer);
            }
        }

        if self.image != 0 {
            unsafe {
                gl::DeleteTextures(1, &self.image);
            }
        }
    }
}

impl Framebuffer {
    pub fn new(max_levels: u32) -> Framebuffer {
        let mut framebuffer = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
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

    fn init(&mut self, mut size: Size, mut format: ShaderFormat) {
        if format == ShaderFormat::Unknown {
            format = ShaderFormat::R8G8B8A8Unorm;
        }

        self.format = GLenum::from(format);
        self.size = size;

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer);

            // reset the framebuffer image
            if self.image != 0 {
                gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, 0, 0);
                gl::DeleteTextures(1, &self.image);
            }

            gl::GenTextures(1, &mut self.image);
            gl::BindTexture(1, self.image);

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

                        gl::TexStorage2D(gl::TEXTURE_2D, self.levels as GLsizei, gl::RGBA8, size.width as GLsizei, size.height as GLsizei);
                        gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                                 gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, self.image, 0);
                        self.init = gl::CheckFramebufferStatus(gl::FRAMEBUFFER) == gl::FRAMEBUFFER_COMPLETE;
                    }
                    _ => panic!("failed to complete: {status}")
                }
            } else {
                self.init = true;
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}
