use std::any::Any;
use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum LibrashaderError {
    #[error("The parameter was null or invalid.")]
    InvalidParameter(&'static str),
    #[error("The path was invalid.")]
    InvalidPath(#[from] std::str::Utf8Error),
    #[error("There was an error parsing the preset.")]
    PresetError(#[from] librashader::presets::ParsePresetError),
    #[error("There was an error preprocessing the shader source.")]
    PreprocessError(#[from] librashader::preprocess::PreprocessError),

    // #[cfg(feature = "runtime-opengl")]
    #[error("There was an error in the OpenGL filter chain.")]
    OpenGlFilterError(#[from] librashader::runtime::gl::error::FilterChainError),
    #[error("There was an unknown error.")]
    UnknownError(Box<dyn Any + Send + 'static>)
}

impl LibrashaderError {
    pub const fn ok() -> libra_error_t {
        std::ptr::null()
    }

    pub fn panic(panic: Box<dyn Any + Send + 'static>) -> libra_error_t {
        LibrashaderError::UnknownError(panic).export()
    }

    pub fn export(self) -> libra_error_t {
        Box::into_raw(Box::new(self))
    }
}

macro_rules! assert_non_null {
    ($value:ident) => {
        if $value.is_null() {
            return $crate::error::LibrashaderError::InvalidParameter(stringify!($value)).export()
        }
    }
}
macro_rules! assert_some {
    ($value:ident) => {
        if $value.is_none() {
            return $crate::error::LibrashaderError::InvalidParameter(stringify!($value)).export()
        }
    }
}

pub(crate) use assert_non_null;
pub(crate) use assert_some;
use crate::ctypes::libra_error_t;