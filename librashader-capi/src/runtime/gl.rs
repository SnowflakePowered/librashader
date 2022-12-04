use std::ffi::{c_char, c_void, CString};
use std::mem::MaybeUninit;
use crate::ctypes::{libra_error_t, libra_gl_filter_chain_t, libra_shader_preset_t, libra_viewport_t};
use crate::error::{assert_non_null, assert_some_ptr, LibrashaderError};
use crate::ffi::ffi_body;
use std::ptr::NonNull;
use librashader::runtime::FilterChain;
use librashader::runtime::gl::{GLImage, Viewport};

pub use librashader::runtime::gl::options::FilterChainOptionsGL;
use librashader::Size;

pub type gl_loader_t = unsafe extern "C" fn (*const c_char) -> *const c_void;
/// Initialize the OpenGL Context for librashader.
///
/// ## Safety
/// Attempting to create a filter chain will fail.
///
/// Reinitializing the OpenGL context with a different loader immediately invalidates previous filter
/// chain objects, and drawing with them causes immediate undefined behaviour.
#[no_mangle]
pub unsafe extern "C" fn libra_gl_init_context(loader: gl_loader_t) -> libra_error_t {
    gl::load_with(|s| {
        unsafe {
            let proc_name = CString::new(s).unwrap_unchecked();
            loader(proc_name.as_ptr())
        }
    });

    LibrashaderError::ok()
}

/// Create the filter chain given the shader preset.
///
/// The shader preset is immediately invalidated and must be recreated after
/// the filter chain is created.
///
/// ## Safety:
/// - `preset` must be either null, or valid and aligned.
/// - `options` must be either null, or valid and aligned.
/// - `out` may be either null or uninitialized, but must be aligned.
#[no_mangle]
pub unsafe extern "C" fn libra_gl_filter_chain_create(preset: *mut libra_shader_preset_t,
                                                      options: *const FilterChainOptionsGL,
                                                      out: *mut MaybeUninit<libra_gl_filter_chain_t>) -> libra_error_t {
    ffi_body!({
        assert_non_null!(preset);
       let preset = unsafe {
            let preset_ptr = &mut *preset;
            let preset = preset_ptr.take();
            Box::from_raw(preset.unwrap().as_ptr())
        };

        let options = if options.is_null() {
            None
        } else {
            Some(unsafe { &*options })
        };


        let chain = librashader::runtime::gl::FilterChainGL::load_from_preset(*preset, options)?;

        unsafe {
            out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(chain)))))
        }
    })
}

#[repr(C)]
pub struct libra_source_image_gl_t {
    pub handle: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32
}

#[repr(C)]
pub struct libra_draw_framebuffer_gl_t {
    pub handle: u32,
    pub texture: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32
}

impl From<libra_source_image_gl_t> for GLImage {
    fn from(value: libra_source_image_gl_t) -> Self {
        GLImage {
            handle: value.handle,
            format: value.format,
            size: Size::new(value.width, value.height),
            padded_size: Size::default()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn libra_gl_filter_chain_frame(chain: *mut libra_gl_filter_chain_t,
                                                     frame_count: usize,
                                                     image: libra_source_image_gl_t,
                                                     viewport: libra_viewport_t,
                                                     out: libra_draw_framebuffer_gl_t,
                                                     mvp: *const f32,

) -> libra_error_t {
    ffi_body!(mut |chain| {
        assert_some_ptr!(mut chain);

        let image: GLImage = image.into();
        let viewport = Viewport {
            x: viewport.x,
            y: viewport.y,
            output: &chain.create_framebuffer_raw(out.texture, out.handle, out.format, Size::new(out.width, out.height), 1),
            mvp: None,
        };
        chain.frame(&image, &viewport, frame_count, None)?;
    })
}


#[no_mangle]
pub unsafe extern "C" fn libra_gl_filter_chain_free(chain: *mut libra_gl_filter_chain_t) -> libra_error_t {
    ffi_body!({
        assert_non_null!(chain);
        unsafe {
            let chain_ptr = &mut *chain;
            let chain = chain_ptr.take();
            drop(Box::from_raw(chain.unwrap().as_ptr()))
        };
    })
}