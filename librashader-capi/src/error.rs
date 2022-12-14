//! The librashader error C API. (`libra_error_*`).
use std::any::Any;
use std::ffi::{c_char, CStr, CString};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use thiserror::Error;

/// The error type for librashader.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum LibrashaderError {
    #[error("There was an unknown error.")]
    UnknownError(Box<dyn Any + Send + 'static>),
    #[error("The parameter was null or invalid.")]
    InvalidParameter(&'static str),
    #[error("The path was invalid.")]
    InvalidPath(#[from] std::str::Utf8Error),
    #[error("There was an error parsing the preset.")]
    PresetError(#[from] librashader::presets::ParsePresetError),
    #[error("There was an error preprocessing the shader source.")]
    PreprocessError(#[from] librashader::preprocess::PreprocessError),
    #[error("There was an error compiling the shader source.")]
    ShaderCompileError(#[from] librashader::reflect::ShaderCompileError),
    #[error("There was an error reflecting the shader source.")]
    ShaderReflectError(#[from] librashader::reflect::ShaderReflectError),
    #[cfg(feature = "runtime-opengl")]
    #[error("There was an error in the OpenGL filter chain.")]
    OpenGlFilterError(#[from] librashader::runtime::gl::error::FilterChainError),
    #[cfg(feature = "runtime-d3d11")]
    #[error("There was an error in the D3D11 filter chain.")]
    D3D11FilterError(#[from] librashader::runtime::d3d11::error::FilterChainError),
}

/// Error codes for librashader error types.
#[repr(i32)]
pub enum LIBRA_ERRNO {
    UNKNOWN_ERROR = 0,
    INVALID_PARAMETER = 1,
    INVALID_PATH = 2,
    PRESET_ERROR = 3,
    PREPROCESS_ERROR = 4,
    RUNTIME_ERROR = 5,
}

pub type PFN_lbr_error_errno = extern "C" fn(error: libra_error_t) -> LIBRA_ERRNO;
#[no_mangle]
/// Get the error code corresponding to this error object.
///
/// ## Safety
///   - `error` must be valid and initialized.
pub extern "C" fn libra_error_errno(error: libra_error_t) -> LIBRA_ERRNO {
    let Some(error) = error else {
        return LIBRA_ERRNO::UNKNOWN_ERROR
    };

    unsafe { error.as_ref().get_code() }
}

pub type PFN_lbr_error_print = extern "C" fn(error: libra_error_t) -> i32;
#[no_mangle]
/// Print the error message.
///
/// If `error` is null, this function does nothing and returns 1. Otherwise, this function returns 0.
/// ## Safety
///   - `error` must be a valid and initialized instance of `libra_error_t`.
pub extern "C" fn libra_error_print(error: libra_error_t) -> i32 {
    let Some(error) = error else {
        return 1
    };
    unsafe {
        let error = error.as_ref();
        println!("{error:?}: {error}");
    }
    return 0;
}

pub type PFN_lbr_error_free = extern "C" fn(error: *mut libra_error_t) -> i32;
#[no_mangle]
/// Frees any internal state kept by the error.
///
/// If `error` is null, this function does nothing and returns 1. Otherwise, this function returns 0.
/// The resulting error object becomes null.
/// ## Safety
///   - `error` must be null or a pointer to a valid and initialized instance of `libra_error_t`.
pub extern "C" fn libra_error_free(error: *mut libra_error_t) -> i32 {
    if error.is_null() {
        return 1;
    }

    let mut error = unsafe { &mut *error };
    let error = error.take();
    let Some(error) = error else {
        return 1;
    };

    unsafe { drop(Box::from_raw(error.as_ptr())) }
    return 0;
}

pub type PFN_lbr_error_write =
    extern "C" fn(error: libra_error_t, out: *mut MaybeUninit<*mut c_char>) -> i32;
#[no_mangle]
/// Writes the error message into `out`
///
/// If `error` is null, this function does nothing and returns 1. Otherwise, this function returns 0.
/// ## Safety
///   - `error` must be a valid and initialized instance of `libra_error_t`.
///   - `out` must be a non-null pointer. The resulting string must not be modified.
pub extern "C" fn libra_error_write(
    error: libra_error_t,
    out: *mut MaybeUninit<*mut c_char>,
) -> i32 {
    let Some(error) = error else {
        return 1
    };
    if out.is_null() {
        return 1;
    }

    unsafe {
        let error = error.as_ref();
        let Ok(cstring) = CString::new(format!("{error:?}: {error}")) else {
            return 1
        };

        out.write(MaybeUninit::new(cstring.into_raw()))
    }
    return 0;
}

pub type PFN_lbr_error_free_string = extern "C" fn(out: *mut *mut c_char) -> i32;
#[no_mangle]
/// Frees an error string previously allocated by `libra_error_write`.
///
/// After freeing, the pointer will be set to null.
/// ## Safety
///   - If `libra_error_write` is not null, it must point to a string previously returned by `libra_error_write`.
///     Attempting to free anything else, including strings or objects from other librashader functions, is immediate
///     Undefined Behaviour.
pub extern "C" fn libra_error_free_string(out: *mut *mut c_char) -> i32 {
    if out.is_null() {
        return 1;
    }

    unsafe {
        let ptr = out.read();
        *out = std::ptr::null_mut();
        drop(CString::from_raw(ptr))
    }
    return 0;
}

impl LibrashaderError {
    pub(crate) const fn get_code(&self) -> LIBRA_ERRNO {
        match self {
            LibrashaderError::UnknownError(_) => LIBRA_ERRNO::UNKNOWN_ERROR,
            LibrashaderError::InvalidParameter(_) => LIBRA_ERRNO::INVALID_PARAMETER,
            LibrashaderError::InvalidPath(_) => LIBRA_ERRNO::INVALID_PATH,
            LibrashaderError::PresetError(_) => LIBRA_ERRNO::PRESET_ERROR,
            LibrashaderError::PreprocessError(_) => LIBRA_ERRNO::PREPROCESS_ERROR,
            LibrashaderError::ShaderCompileError(_) | LibrashaderError::ShaderReflectError(_) => {
                LIBRA_ERRNO::RUNTIME_ERROR
            }
            #[cfg(feature = "runtime-opengl")]
            LibrashaderError::OpenGlFilterError(_) => LIBRA_ERRNO::RUNTIME_ERROR,
            #[cfg(feature = "runtime-d3d11")]
            LibrashaderError::D3D11FilterError(_) => LIBRA_ERRNO::RUNTIME_ERROR,
        }
    }
    pub(crate) const fn ok() -> libra_error_t {
        None
    }

    pub(crate) fn panic(panic: Box<dyn Any + Send + 'static>) -> libra_error_t {
        LibrashaderError::UnknownError(panic).export()
    }

    pub(crate) fn export(self) -> libra_error_t {
        NonNull::new(Box::into_raw(Box::new(self)))
    }
}

macro_rules! assert_non_null {
    ($value:ident) => {
        if $value.is_null() {
            return $crate::error::LibrashaderError::InvalidParameter(stringify!($value)).export();
        }
    };
    (noexport $value:ident) => {
        if $value.is_null() {
            return Err($crate::error::LibrashaderError::InvalidParameter(
                stringify!($value),
            ));
        }
    };
}
macro_rules! assert_some {
    ($value:ident) => {
        if $value.is_none() {
            return $crate::error::LibrashaderError::InvalidParameter(stringify!($value)).export();
        }
    };
}

macro_rules! assert_some_ptr {
    ($value:ident) => {
        if $value.is_none() {
            return $crate::error::LibrashaderError::InvalidParameter(stringify!($value)).export();
        }

        let $value = unsafe { $value.as_ref().unwrap().as_ref() };
    };
    (mut $value:ident) => {
        if $value.is_none() {
            return $crate::error::LibrashaderError::InvalidParameter(stringify!($value)).export();
        }

        let $value = unsafe { $value.as_mut().unwrap().as_mut() };
    };
}

use crate::ctypes::libra_error_t;
pub(crate) use assert_non_null;
pub(crate) use assert_some;
pub(crate) use assert_some_ptr;
