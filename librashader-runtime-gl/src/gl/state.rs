use crate::filter_chain::GLCaps;
use glow::HasContext;

macro_rules! backup_gl_int_state {
    (let $var:ident as $glenum:expr; $gl:ident) => {
        // GL object names and enums are non-negative; store them as u32 so the
        // restore path needs no casts.
        let $var = $gl.get_parameter_i32($glenum) as u32;
    };
    (let $var:ident as $glenum:expr; if $if:expr; $gl:ident) => {
        let $var = if $if {
            $gl.get_parameter_i32($glenum) as u32
        } else {
            0
        };
    };
    (let $var:ident[$num:literal] as $glenum:expr; $gl:ident) => {
        let mut $var = [0; $num];
        $gl.get_parameter_i32_slice($glenum, &mut $var);
    };
    (let $var:ident[$num:literal] as $glenum:expr; if $if:expr; $gl:ident) => {
        let mut $var = [0; $num];
        if $if {
            $gl.get_parameter_i32_slice($glenum, &mut $var);
        }
    };
    (let enabled $var:ident as $glenum:expr; $gl:ident) => {
        let $var = $gl.is_enabled($glenum);
    };
    (let enabled $var:ident as $glenum:expr; if $if:expr; $gl:ident) => {
        let $var = if $if { $gl.is_enabled($glenum) } else { false };
    };
}

/// Guard to prepare the fixed function state for librashader rendering
pub struct EnterFixedFunctionState<'gl> {
    gl: &'gl glow::Context,
    last_enable_blend: bool,
    last_enable_cull_face: bool,
    last_enable_depth_test: bool,
    last_enable_stencil_test: bool,
    last_enable_scissor_test: bool,
    last_enable_primitive_restart: bool,
    last_enable_framebuffer_srgb: bool,

    has_primitive_restart: bool,
    has_framebuffer_srgb: bool,
}

impl<'gl> EnterFixedFunctionState<'gl> {
    /// Backs up fixed function state, then
    /// disables SCISSOR_TEST/CULL_FACE/BLEND/DEPTH_TEST/STENCIL_TEST
    /// to prepare the frame for rendering.
    pub fn new(gl: &'gl glow::Context, caps: &GLCaps) -> Self {
        unsafe {
            backup_gl_int_state!(let enabled last_enable_blend as glow::BLEND; gl);
            backup_gl_int_state!(let enabled last_enable_cull_face as glow::CULL_FACE; gl);
            backup_gl_int_state!(let enabled last_enable_depth_test as glow::DEPTH_TEST; gl);
            backup_gl_int_state!(let enabled last_enable_stencil_test as glow::STENCIL_TEST; gl);
            backup_gl_int_state!(let enabled last_enable_scissor_test as glow::SCISSOR_TEST; gl);

            backup_gl_int_state!(let enabled last_enable_primitive_restart as glow::PRIMITIVE_RESTART;
                    if caps.primitive_restart; gl);
            backup_gl_int_state!(let enabled last_enable_framebuffer_srgb as glow::FRAMEBUFFER_SRGB;
                    if caps.framebuffer_srgb; gl);

            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::STENCIL_TEST);

            EnterFixedFunctionState {
                gl,
                last_enable_blend,
                last_enable_cull_face,
                last_enable_depth_test,
                last_enable_stencil_test,
                last_enable_scissor_test,
                last_enable_primitive_restart,
                last_enable_framebuffer_srgb,
                has_primitive_restart: caps.primitive_restart,
                has_framebuffer_srgb: caps.framebuffer_srgb,
            }
        }
    }
}

impl Drop for EnterFixedFunctionState<'_> {
    fn drop(&mut self) {
        let gl = self.gl;

        unsafe {
            if self.last_enable_blend {
                gl.enable(glow::BLEND)
            } else {
                gl.disable(glow::BLEND)
            }

            if self.last_enable_cull_face {
                gl.enable(glow::CULL_FACE)
            } else {
                gl.disable(glow::CULL_FACE)
            }

            if self.last_enable_depth_test {
                gl.enable(glow::DEPTH_TEST)
            } else {
                gl.disable(glow::DEPTH_TEST)
            }

            if self.last_enable_stencil_test {
                gl.enable(glow::STENCIL_TEST)
            } else {
                gl.disable(glow::STENCIL_TEST)
            }

            if self.last_enable_scissor_test {
                gl.enable(glow::SCISSOR_TEST)
            } else {
                gl.disable(glow::SCISSOR_TEST)
            }

            if self.has_primitive_restart {
                if self.last_enable_primitive_restart {
                    gl.enable(glow::PRIMITIVE_RESTART)
                } else {
                    gl.disable(glow::PRIMITIVE_RESTART)
                }
            }

            if self.has_framebuffer_srgb {
                if self.last_enable_framebuffer_srgb {
                    gl.enable(glow::FRAMEBUFFER_SRGB)
                } else {
                    gl.disable(glow::FRAMEBUFFER_SRGB)
                }
            }
        }
    }
}
