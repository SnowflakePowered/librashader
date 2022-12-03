use std::ffi::{c_char, c_void, CString};
use std::mem::MaybeUninit;
use crate::ctypes::{libra_error_t, libra_gl_filter_chain_t, libra_shader_preset_t};
use crate::error::{assert_non_null, assert_some, LibrashaderError};
use crate::ffi::ffi_body;
use std::mem::ManuallyDrop;

pub use librashader::runtime::gl::options::FilterChainOptionsGL;

pub type gl_loader_t = unsafe extern "C" fn (*const c_char) -> *const c_void;
/// Initialize the OpenGL Context for librashader.
///
/// ## Safety
/// Attempting to create a filter chain before initializing the GL context is undefined behaviour.
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
pub unsafe extern "C" fn libra_gl_create_filter_chain(preset: *mut libra_shader_preset_t,
                                                      options: *const FilterChainOptionsGL,
                                                      out: *mut MaybeUninit<libra_gl_filter_chain_t>) -> libra_error_t {
    ffi_body!({
        assert_non_null!(preset);
        let preset_ptr = unsafe {
            &mut *preset
        };

        assert_some!(preset_ptr);
        let preset = preset_ptr.take().unwrap();
        let options = if options.is_null() {
            None
        } else {
            Some(unsafe { &*options })
        };
        let chain = librashader::runtime::gl::FilterChainGL::load_from_preset(*preset, options)?;
        unsafe {
            out.write(MaybeUninit::new(ManuallyDrop::new(Some(Box::new(chain)))))
        }
    })
}