use crate::filter_chain::GLCaps;
use glow::HasContext;
use std::num::NonZeroU32;

/// The texture and sampler bound to a single texture image unit.
#[derive(Copy, Clone, Default)]
struct TextureUnitBinding {
    texture: u32,
    sampler: u32,
}

pub struct StateBackup<'gl> {
    gl: &'gl glow::Context,
    last_active_texture: u32,
    last_program: u32,
    // The texture and sampler bound to each unit `[0, texture_units)` that the
    // filter chain may clobber. The number of units depends on the preset.
    last_texture_units: Vec<TextureUnitBinding>,
    last_array_buffer: u32,
    last_vertex_array: u32,
    last_polygon_mode: [i32; 2],
    last_viewport: [i32; 4],
    last_scissor_box: [i32; 4],
    last_blend_src_rgb: u32,
    last_blend_dst_rgb: u32,
    last_blend_src_alpha: u32,
    last_blend_dst_alpha: u32,
    last_blend_equation_rgb: u32,
    last_blend_equation_alpha: u32,
    last_draw_framebuffer: u32,
    last_read_framebuffer: u32,

    caps: GLCaps,
}

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

impl<'gl> StateBackup<'gl> {
    /// Back up the GL state before running frame.
    /// After this returns, the active texture will be set to `active_texture`
    ///
    /// `texture_units` is the number of textures and samplers to change.
    pub fn new(
        gl: &'gl glow::Context,
        caps: GLCaps,
        active_texture: u32,
        texture_units: u32,
    ) -> Self {
        unsafe {
            backup_gl_int_state!(let last_active_texture as glow::ACTIVE_TEXTURE; gl);

            let mut last_texture_units =
                vec![TextureUnitBinding::default(); texture_units as usize];

            for (unit, binding) in last_texture_units.iter_mut().enumerate() {
                gl.active_texture(glow::TEXTURE0 + unit as u32);
                binding.texture = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D) as u32;
                if caps.sampler {
                    binding.sampler = gl.get_parameter_i32(glow::SAMPLER_BINDING) as u32;
                }
            }
            gl.active_texture(active_texture);

            backup_gl_int_state!(let last_program as glow::CURRENT_PROGRAM; gl);
            backup_gl_int_state!(let last_array_buffer as glow::ARRAY_BUFFER_BINDING; gl);

            backup_gl_int_state!(let last_vertex_array as glow::VERTEX_ARRAY_BINDING;
                if caps.vertex_array; gl);
            backup_gl_int_state!(let last_polygon_mode[2] as glow::POLYGON_MODE;
                if caps.polygon_mode; gl);

            backup_gl_int_state!(let last_viewport[4] as glow::VIEWPORT; gl);
            backup_gl_int_state!(let last_scissor_box[4] as glow::SCISSOR_BOX; gl);

            backup_gl_int_state!(let last_blend_src_rgb as glow::BLEND_SRC_RGB; gl);
            backup_gl_int_state!(let last_blend_dst_rgb as glow::BLEND_DST_RGB; gl);

            backup_gl_int_state!(let last_blend_src_alpha as glow::BLEND_SRC_ALPHA; gl);
            backup_gl_int_state!(let last_blend_dst_alpha as glow::BLEND_DST_ALPHA; gl);

            backup_gl_int_state!(let last_blend_equation_rgb as glow::BLEND_EQUATION_RGB; gl);
            backup_gl_int_state!(let last_blend_equation_alpha as glow::BLEND_EQUATION_ALPHA; gl);

            // GL_FRAMEBUFFER_BINDING aliases GL_DRAW_FRAMEBUFFER_BINDING, so this is valid
            // even when separate draw/read targets are unavailable.
            backup_gl_int_state!(let last_draw_framebuffer as glow::DRAW_FRAMEBUFFER_BINDING; gl);
            backup_gl_int_state!(let last_read_framebuffer as glow::READ_FRAMEBUFFER_BINDING;
                if caps.separate_framebuffer; gl);

            StateBackup {
                gl,
                last_active_texture,
                last_program,
                last_texture_units,
                last_array_buffer,
                last_vertex_array,
                last_polygon_mode,
                last_viewport,
                last_scissor_box,
                last_blend_src_rgb,
                last_blend_dst_rgb,
                last_blend_src_alpha,
                last_blend_dst_alpha,
                last_blend_equation_rgb,
                last_blend_equation_alpha,
                last_draw_framebuffer,
                last_read_framebuffer,
                caps,
            }
        }
    }
}

impl Drop for StateBackup<'_> {
    fn drop(&mut self) {
        let gl = self.gl;
        unsafe {
            gl.use_program(NonZeroU32::new(self.last_program).map(glow::NativeProgram));

            // Restore the texture and sampler bound to each unit the chain may have
            // clobbered, then leave the originally active unit selected.
            for (unit, binding) in self.last_texture_units.iter().enumerate() {
                gl.active_texture(glow::TEXTURE0 + unit as u32);
                gl.bind_texture(
                    glow::TEXTURE_2D,
                    NonZeroU32::new(binding.texture).map(glow::NativeTexture),
                );
                if self.caps.sampler {
                    gl.bind_sampler(
                        unit as u32,
                        NonZeroU32::new(binding.sampler).map(glow::NativeSampler),
                    );
                }
            }
            gl.active_texture(self.last_active_texture);

            if self.caps.vertex_array {
                gl.bind_vertex_array(
                    NonZeroU32::new(self.last_vertex_array).map(glow::NativeVertexArray),
                );
            }

            gl.bind_buffer(
                glow::ARRAY_BUFFER,
                NonZeroU32::new(self.last_array_buffer).map(glow::NativeBuffer),
            );

            if self.caps.separate_framebuffer {
                gl.bind_framebuffer(
                    glow::DRAW_FRAMEBUFFER,
                    NonZeroU32::new(self.last_draw_framebuffer).map(glow::NativeFramebuffer),
                );
                gl.bind_framebuffer(
                    glow::READ_FRAMEBUFFER,
                    NonZeroU32::new(self.last_read_framebuffer).map(glow::NativeFramebuffer),
                );
            } else {
                gl.bind_framebuffer(
                    glow::FRAMEBUFFER,
                    NonZeroU32::new(self.last_draw_framebuffer).map(glow::NativeFramebuffer),
                );
            }

            gl.blend_equation_separate(
                self.last_blend_equation_rgb,
                self.last_blend_equation_alpha,
            );
            gl.blend_func_separate(
                self.last_blend_src_rgb,
                self.last_blend_dst_rgb,
                self.last_blend_src_alpha,
                self.last_blend_dst_alpha,
            );

            if self.caps.polygon_mode {
                gl.polygon_mode(glow::FRONT_AND_BACK, self.last_polygon_mode[0] as u32);
            }

            gl.viewport(
                self.last_viewport[0],
                self.last_viewport[1],
                self.last_viewport[2],
                self.last_viewport[3],
            );
            gl.scissor(
                self.last_scissor_box[0],
                self.last_scissor_box[1],
                self.last_scissor_box[2],
                self.last_scissor_box[3],
            );
        }
    }
}

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
