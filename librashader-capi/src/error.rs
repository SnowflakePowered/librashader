//! librashader error C API. (`libra_error_*`).
use std::any::Any;
use std::ffi::{c_char, CString};
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;
use thiserror::Error;

/// The error type for librashader C API.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum LibrashaderError {
    /// An unknown error or panic occurred.
    #[error("There was an unknown error.")]
    UnknownError(Box<dyn Any + Send + 'static>),

    /// An invalid parameter (likely null), was passed.
    #[error("The parameter was null or invalid.")]
    InvalidParameter(&'static str),

    /// The string provided was not valid UTF-8.
    #[error("The provided string was not valid UTF8.")]
    InvalidString(#[from] std::str::Utf8Error),

    /// An error occurred in the preset parser.
    #[error("There was an error parsing the preset.")]
    PresetError(#[from] librashader::presets::ParsePresetError),

    /// An error occurred in the shader preprocessor.
    #[error("There was an error preprocessing the shader source.")]
    PreprocessError(#[from] librashader::preprocess::PreprocessError),

    /// An error occurred in the shader compiler.
    #[error("There was an error compiling the shader source.")]
    ShaderCompileError(#[from] librashader::reflect::ShaderCompileError),

    /// An error occrred when validating and reflecting the shader.
    #[error("There was an error reflecting the shader source.")]
    ShaderReflectError(#[from] librashader::reflect::ShaderReflectError),

    /// An invalid shader parameter name was provided.
    #[error("The provided parameter name was invalid.")]
    UnknownShaderParameter(*const c_char),

    /// An error occurred with the OpenGL filter chain.
    #[cfg(feature = "runtime-opengl")]
    #[cfg_attr(feature = "docsrs", doc(cfg(feature = "runtime-opengl")))]
    #[error("There was an error in the OpenGL filter chain.")]
    OpenGlFilterError(#[from] librashader::runtime::gl::error::FilterChainError),

    /// An error occurred with the Direct3D 11 filter chain.
    #[cfg(all(target_os = "windows", feature = "runtime-d3d11"))]
    #[cfg_attr(
        feature = "docsrs",
        doc(cfg(all(target_os = "windows", feature = "runtime-d3d11")))
    )]
    #[error("There was an error in the D3D11 filter chain.")]
    D3D11FilterError(#[from] librashader::runtime::d3d11::error::FilterChainError),

    /// An error occurred with the Direct3D 12 filter chain.
    #[cfg(all(target_os = "windows", feature = "runtime-d3d12"))]
    #[cfg_attr(
        feature = "docsrs",
        doc(cfg(all(target_os = "windows", feature = "runtime-d3d12")))
    )]
    #[error("There was an error in the D3D12 filter chain.")]
    D3D12FilterError(#[from] librashader::runtime::d3d12::error::FilterChainError),

    /// An error occurred with the Direct3D 9 filter chain.
    #[cfg(all(target_os = "windows", feature = "runtime-d3d9"))]
    #[cfg_attr(
        feature = "docsrs",
        doc(cfg(all(target_os = "windows", feature = "runtime-d3d9")))
    )]
    #[error("There was an error in the D3D9 filter chain.")]
    D3D9FilterError(#[from] librashader::runtime::d3d9::error::FilterChainError),

    /// An error occurred with the Vulkan filter chain.

    #[cfg(feature = "runtime-vulkan")]
    #[cfg_attr(feature = "docsrs", doc(cfg(feature = "runtime-vulkan")))]
    #[error("There was an error in the Vulkan filter chain.")]
    VulkanFilterError(#[from] librashader::runtime::vk::error::FilterChainError),

    /// An error occurred with the Metal filter chain.
    #[cfg_attr(
        feature = "docsrs",
        doc(cfg(all(target_vendor = "apple", feature = "runtime-metal")))
    )]
    #[cfg(all(target_vendor = "apple", feature = "runtime-metal"))]
    #[error("There was an error in the Metal filter chain.")]
    MetalFilterError(#[from] librashader::runtime::mtl::error::FilterChainError),
    /// This error is unreachable.
    #[error("This error is not reachable")]
    Infallible(#[from] std::convert::Infallible),
}

/// Error codes for librashader error types.
#[repr(i32)]
pub enum LIBRA_ERRNO {
    /// Error code for an unknown error.
    UNKNOWN_ERROR = 0,

    /// Error code for an invalid parameter.
    INVALID_PARAMETER = 1,

    /// Error code for an invalid (non-UTF8) string.
    INVALID_STRING = 2,

    /// Error code for a preset parser error.
    PRESET_ERROR = 3,

    /// Error code for a preprocessor error.
    PREPROCESS_ERROR = 4,

    /// Error code for a shader parameter error.
    SHADER_PARAMETER_ERROR = 5,

    /// Error code for a reflection error.
    REFLECT_ERROR = 6,

    /// Error code for a runtime error.
    RUNTIME_ERROR = 7,

    /// Error code for when a LUT fails to load at runtime.
    LUT_LOAD_ERROR = 8,

    /// Error code when a shader fails to be compiled by the graphics driver.
    COMPILE_ERROR = 9,
}

// Nothing here can use extern_fn because they are lower level than libra_error_t.

/// Function pointer definition for libra_error_errno
pub type PFN_libra_error_errno = extern "C" fn(error: libra_error_t) -> LIBRA_ERRNO;
#[no_mangle]
/// Get the error code corresponding to this error object.
///
/// ## Safety
///   - `error` must be valid and initialized.
pub unsafe extern "C" fn libra_error_errno(error: libra_error_t) -> LIBRA_ERRNO {
    let Some(error) = error else {
        return LIBRA_ERRNO::UNKNOWN_ERROR;
    };

    unsafe {
        let code = error.as_ref().get_code();

        // Unwrap inner runtime codes if the parsing was done in the
        // runtime crate
        if matches!(code, LIBRA_ERRNO::RUNTIME_ERROR) {
            error.as_ref().get_runtime_code()
        } else {
            code
        }
    }
}

/// Function pointer definition for libra_error_print
pub type PFN_libra_error_print = extern "C" fn(error: libra_error_t) -> i32;
#[no_mangle]
/// Print the error message.
///
/// If `error` is null, this function does nothing and returns 1. Otherwise, this function returns 0.
/// ## Safety
///   - `error` must be a valid and initialized instance of `libra_error_t`.
pub unsafe extern "C" fn libra_error_print(error: libra_error_t) -> i32 {
    let Some(error) = error else { return 1 };
    unsafe {
        let error = error.as_ref();
        println!("{error:?}: {error}");
    }
    0
}

/// Function pointer definition for libra_error_free
pub type PFN_libra_error_free = extern "C" fn(error: *mut libra_error_t) -> i32;
#[no_mangle]
/// Frees any internal state kept by the error.
///
/// If `error` is null, this function does nothing and returns 1. Otherwise, this function returns 0.
/// The resulting error object becomes null.
/// ## Safety
///   - `error` must be null or a pointer to a valid and initialized instance of `libra_error_t`.
pub unsafe extern "C" fn libra_error_free(error: *mut libra_error_t) -> i32 {
    if error.is_null() {
        return 1;
    }

    let error = unsafe { &mut *error };
    let error = error.take();
    let Some(error) = error else {
        return 1;
    };

    unsafe { drop(Box::from_raw(error.as_ptr())) }
    0
}

/// Function pointer definition for libra_error_write
pub type PFN_libra_error_write =
    extern "C" fn(error: libra_error_t, out: *mut MaybeUninit<*mut c_char>) -> i32;
#[no_mangle]
/// Writes the error message into `out`
///
/// If `error` is null, this function does nothing and returns 1. Otherwise, this function returns 0.
/// ## Safety
///   - `error` must be a valid and initialized instance of `libra_error_t`.
///   - `out` must be a non-null pointer. The resulting string must not be modified.
pub unsafe extern "C" fn libra_error_write(
    error: libra_error_t,
    out: *mut MaybeUninit<*mut c_char>,
) -> i32 {
    let Some(error) = error else { return 1 };
    if out.is_null() {
        return 1;
    }

    unsafe {
        let error = error.as_ref();
        let Ok(cstring) = CString::new(format!("{error:?}: {error}")) else {
            return 1;
        };

        out.write(MaybeUninit::new(cstring.into_raw()))
    }
    0
}

/// Function pointer definition for libra_error_free_string
pub type PFN_libra_error_free_string = extern "C" fn(out: *mut *mut c_char) -> i32;
#[no_mangle]
/// Frees an error string previously allocated by `libra_error_write`.
///
/// After freeing, the pointer will be set to null.
/// ## Safety
///   - If `libra_error_write` is not null, it must point to a string previously returned by `libra_error_write`.
///     Attempting to free anything else, including strings or objects from other librashader functions, is immediate
///     Undefined Behaviour.
pub unsafe extern "C" fn libra_error_free_string(out: *mut *mut c_char) -> i32 {
    if out.is_null() {
        return 1;
    }

    unsafe {
        let ptr = out.read();
        *out = std::ptr::null_mut();
        drop(CString::from_raw(ptr))
    }
    0
}

impl LibrashaderError {
    pub(crate) const fn get_code(&self) -> LIBRA_ERRNO {
        match self {
            LibrashaderError::UnknownError(_) => LIBRA_ERRNO::UNKNOWN_ERROR,
            LibrashaderError::InvalidParameter(_) => LIBRA_ERRNO::INVALID_PARAMETER,
            LibrashaderError::InvalidString(_) => LIBRA_ERRNO::INVALID_STRING,
            LibrashaderError::PresetError(_) => LIBRA_ERRNO::PRESET_ERROR,
            LibrashaderError::PreprocessError(_) => LIBRA_ERRNO::PREPROCESS_ERROR,
            LibrashaderError::ShaderCompileError(_) | LibrashaderError::ShaderReflectError(_) => {
                LIBRA_ERRNO::REFLECT_ERROR
            }
            LibrashaderError::UnknownShaderParameter(_) => LIBRA_ERRNO::SHADER_PARAMETER_ERROR,
            #[cfg(feature = "runtime-opengl")]
            LibrashaderError::OpenGlFilterError(_) => LIBRA_ERRNO::RUNTIME_ERROR,
            #[cfg(all(target_os = "windows", feature = "runtime-d3d11"))]
            LibrashaderError::D3D11FilterError(_) => LIBRA_ERRNO::RUNTIME_ERROR,
            #[cfg(all(target_os = "windows", feature = "runtime-d3d12"))]
            LibrashaderError::D3D12FilterError(_) => LIBRA_ERRNO::RUNTIME_ERROR,
            #[cfg(all(target_os = "windows", feature = "runtime-d3d9"))]
            LibrashaderError::D3D9FilterError(_) => LIBRA_ERRNO::RUNTIME_ERROR,
            #[cfg(feature = "runtime-vulkan")]
            LibrashaderError::VulkanFilterError(_) => LIBRA_ERRNO::RUNTIME_ERROR,
            #[cfg(all(target_vendor = "apple", feature = "runtime-metal"))]
            LibrashaderError::MetalFilterError(_) => LIBRA_ERRNO::RUNTIME_ERROR,
            LibrashaderError::Infallible(_) => LIBRA_ERRNO::UNKNOWN_ERROR,
        }
    }

    /// Get the inner code of the runtime error
    pub(crate) const fn get_runtime_code(&self) -> LIBRA_ERRNO {
        match self {
            #[cfg(feature = "runtime-opengl")]
            LibrashaderError::OpenGlFilterError(ogl) => match ogl {
                librashader::runtime::gl::error::FilterChainError::FramebufferInit(_) => {
                    LIBRA_ERRNO::RUNTIME_ERROR
                }
                librashader::runtime::gl::error::FilterChainError::SpirvCrossReflectError(_) => {
                    LIBRA_ERRNO::REFLECT_ERROR
                }
                librashader::runtime::gl::error::FilterChainError::ShaderPresetError(_) => {
                    LIBRA_ERRNO::PRESET_ERROR
                }
                librashader::runtime::gl::error::FilterChainError::ShaderPreprocessError(_) => {
                    LIBRA_ERRNO::PREPROCESS_ERROR
                }
                librashader::runtime::gl::error::FilterChainError::ShaderCompileError(_)
                | librashader::runtime::gl::error::FilterChainError::ShaderReflectError(_) => {
                    LIBRA_ERRNO::REFLECT_ERROR
                }
                librashader::runtime::gl::error::FilterChainError::LutLoadError(_) => {
                    LIBRA_ERRNO::LUT_LOAD_ERROR
                }
                librashader::runtime::gl::error::FilterChainError::GLLoadError => {
                    LIBRA_ERRNO::RUNTIME_ERROR
                }
                librashader::runtime::gl::error::FilterChainError::GLLinkError(_)
                | librashader::runtime::gl::error::FilterChainError::GlCompileError(_)
                | librashader::runtime::gl::error::FilterChainError::GlProgramError(_) => {
                    LIBRA_ERRNO::COMPILE_ERROR
                }
                librashader::runtime::gl::error::FilterChainError::GlSamplerError
                | librashader::runtime::gl::error::FilterChainError::GlInvalidFramebuffer
                | librashader::runtime::gl::error::FilterChainError::GlError(_) => {
                    LIBRA_ERRNO::RUNTIME_ERROR
                }
                librashader::runtime::gl::error::FilterChainError::Infallible(_) => {
                    LIBRA_ERRNO::UNKNOWN_ERROR
                }
                _ => LIBRA_ERRNO::RUNTIME_ERROR,
            },
            #[cfg(all(target_os = "windows", feature = "runtime-d3d11"))]
            LibrashaderError::D3D11FilterError(d3d) => match d3d {
                librashader::runtime::d3d11::error::FilterChainError::Direct3DOperationError(_)
                | librashader::runtime::d3d11::error::FilterChainError::Direct3DError(_) => {
                    LIBRA_ERRNO::RUNTIME_ERROR
                }
                librashader::runtime::d3d11::error::FilterChainError::D3DCompileError(..) => {
                    LIBRA_ERRNO::COMPILE_ERROR
                }
                librashader::runtime::d3d11::error::FilterChainError::ShaderPresetError(_) => {
                    LIBRA_ERRNO::PRESET_ERROR
                }
                librashader::runtime::d3d11::error::FilterChainError::ShaderPreprocessError(_) => {
                    LIBRA_ERRNO::PREPROCESS_ERROR
                }
                librashader::runtime::d3d11::error::FilterChainError::ShaderCompileError(_)
                | librashader::runtime::d3d11::error::FilterChainError::ShaderReflectError(_) => {
                    LIBRA_ERRNO::REFLECT_ERROR
                }
                librashader::runtime::d3d11::error::FilterChainError::LutLoadError(_) => {
                    LIBRA_ERRNO::LUT_LOAD_ERROR
                }
                _ => LIBRA_ERRNO::RUNTIME_ERROR,
            },
            #[cfg(all(target_os = "windows", feature = "runtime-d3d12"))]
            LibrashaderError::D3D12FilterError(d3d) => match d3d {
                librashader::runtime::d3d12::error::FilterChainError::Direct3DOperationError(_)
                | librashader::runtime::d3d12::error::FilterChainError::Direct3DError(_) => {
                    LIBRA_ERRNO::RUNTIME_ERROR
                }
                librashader::runtime::d3d12::error::FilterChainError::ShaderPresetError(_) => {
                    LIBRA_ERRNO::PRESET_ERROR
                }
                librashader::runtime::d3d12::error::FilterChainError::ShaderPreprocessError(_) => {
                    LIBRA_ERRNO::PREPROCESS_ERROR
                }
                librashader::runtime::d3d12::error::FilterChainError::ShaderCompileError(_)
                | librashader::runtime::d3d12::error::FilterChainError::ShaderReflectError(_) => {
                    LIBRA_ERRNO::REFLECT_ERROR
                }
                librashader::runtime::d3d12::error::FilterChainError::LutLoadError(_) => {
                    LIBRA_ERRNO::LUT_LOAD_ERROR
                }
                librashader::runtime::d3d12::error::FilterChainError::HeapError(_)
                | librashader::runtime::d3d12::error::FilterChainError::AllocationError(_)
                | librashader::runtime::d3d12::error::FilterChainError::InvalidDimensionError(_) => {
                    LIBRA_ERRNO::RUNTIME_ERROR
                }
                librashader::runtime::d3d12::error::FilterChainError::Infallible(_) => {
                    LIBRA_ERRNO::UNKNOWN_ERROR
                }
                _ => LIBRA_ERRNO::RUNTIME_ERROR,
            },
            #[cfg(all(target_os = "windows", feature = "runtime-d3d9"))]
            LibrashaderError::D3D9FilterError(d3d) => match d3d {
                librashader::runtime::d3d9::error::FilterChainError::Direct3DOperationError(_)
                | librashader::runtime::d3d9::error::FilterChainError::Direct3DError(_) => {
                    LIBRA_ERRNO::RUNTIME_ERROR
                }
                librashader::runtime::d3d9::error::FilterChainError::ShaderPresetError(_) => {
                    LIBRA_ERRNO::PRESET_ERROR
                }
                librashader::runtime::d3d9::error::FilterChainError::ShaderPreprocessError(_) => {
                    LIBRA_ERRNO::PREPROCESS_ERROR
                }
                librashader::runtime::d3d9::error::FilterChainError::ShaderCompileError(_)
                | librashader::runtime::d3d9::error::FilterChainError::ShaderReflectError(_) => {
                    LIBRA_ERRNO::REFLECT_ERROR
                }
                librashader::runtime::d3d9::error::FilterChainError::LutLoadError(_) => {
                    LIBRA_ERRNO::LUT_LOAD_ERROR
                }
                librashader::runtime::d3d9::error::FilterChainError::UniformNameError(_) => {
                    LIBRA_ERRNO::INVALID_STRING
                }
                _ => LIBRA_ERRNO::RUNTIME_ERROR,
            },
            #[cfg(feature = "runtime-vulkan")]
            LibrashaderError::VulkanFilterError(vk) => match vk {
                librashader::runtime::vk::error::FilterChainError::HandleIsNull => {
                    LIBRA_ERRNO::INVALID_PARAMETER
                }
                librashader::runtime::vk::error::FilterChainError::ShaderPresetError(_) => {
                    LIBRA_ERRNO::PRESET_ERROR
                }
                librashader::runtime::vk::error::FilterChainError::ShaderPreprocessError(_) => {
                    LIBRA_ERRNO::PREPROCESS_ERROR
                }
                librashader::runtime::vk::error::FilterChainError::ShaderCompileError(_)
                | librashader::runtime::vk::error::FilterChainError::ShaderReflectError(_) => {
                    LIBRA_ERRNO::REFLECT_ERROR
                }
                librashader::runtime::vk::error::FilterChainError::LutLoadError(_) => {
                    LIBRA_ERRNO::LUT_LOAD_ERROR
                }
                librashader::runtime::vk::error::FilterChainError::VulkanResult(_)
                | librashader::runtime::vk::error::FilterChainError::VulkanMemoryError(_)
                | librashader::runtime::vk::error::FilterChainError::AllocationError(_)
                | librashader::runtime::vk::error::FilterChainError::AllocationDoesNotExist => {
                    LIBRA_ERRNO::RUNTIME_ERROR
                }
                librashader::runtime::vk::error::FilterChainError::Infallible(_) => {
                    LIBRA_ERRNO::UNKNOWN_ERROR
                }
                _ => LIBRA_ERRNO::RUNTIME_ERROR,
            },
            #[cfg(all(target_vendor = "apple", feature = "runtime-metal"))]
            LibrashaderError::MetalFilterError(mtl) => match mtl {
                librashader::runtime::mtl::error::FilterChainError::ShaderPresetError(_) => {
                    LIBRA_ERRNO::PRESET_ERROR
                }
                librashader::runtime::mtl::error::FilterChainError::ShaderPreprocessError(_) => {
                    LIBRA_ERRNO::PREPROCESS_ERROR
                }
                librashader::runtime::mtl::error::FilterChainError::ShaderCompileError(_)
                | librashader::runtime::mtl::error::FilterChainError::ShaderReflectError(_) => {
                    LIBRA_ERRNO::REFLECT_ERROR
                }
                librashader::runtime::mtl::error::FilterChainError::LutLoadError(_) => {
                    LIBRA_ERRNO::LUT_LOAD_ERROR
                }
                librashader::runtime::mtl::error::FilterChainError::SamplerError(_, _, _)
                | librashader::runtime::mtl::error::FilterChainError::BufferError
                | librashader::runtime::mtl::error::FilterChainError::MetalError(_)
                | librashader::runtime::mtl::error::FilterChainError::ShaderWrongEntryName
                | librashader::runtime::mtl::error::FilterChainError::FailedToCreateRenderPass
                | librashader::runtime::mtl::error::FilterChainError::FailedToCreateTexture
                | librashader::runtime::mtl::error::FilterChainError::FailedToCreateCommandBuffer => {
                    LIBRA_ERRNO::RUNTIME_ERROR
                }
                librashader::runtime::mtl::error::FilterChainError::Infallible(_) => {
                    LIBRA_ERRNO::UNKNOWN_ERROR
                }
                _ => LIBRA_ERRNO::RUNTIME_ERROR,
            },
            LibrashaderError::Infallible(_) => LIBRA_ERRNO::UNKNOWN_ERROR,
            _ => self.get_code(),
        }
    }

    pub(crate) const fn ok() -> libra_error_t {
        None
    }

    pub(crate) fn export(self) -> libra_error_t {
        NonNull::new(Box::into_raw(Box::new(self)))
    }

    /// Build a [`libra_error_details_t`] describing the innermost cause of this error.
    ///
    /// All strings in the returned struct are owned heap allocations and must be released
    /// via `libra_error_free_details`.
    pub(crate) fn classify(&self) -> libra_error_details_t {
        match self {
            LibrashaderError::PresetError(e) => classify_preset(e),
            LibrashaderError::PreprocessError(e) => classify_preprocess(e),
            LibrashaderError::ShaderCompileError(e) => classify_compile(e),
            LibrashaderError::ShaderReflectError(e) => classify_reflect(e),
            #[cfg(feature = "runtime-opengl")]
            LibrashaderError::OpenGlFilterError(e) => classify_gl(e),
            #[cfg(all(target_os = "windows", feature = "runtime-d3d11"))]
            LibrashaderError::D3D11FilterError(e) => classify_d3d11(e),
            #[cfg(all(target_os = "windows", feature = "runtime-d3d12"))]
            LibrashaderError::D3D12FilterError(e) => classify_d3d12(e),
            #[cfg(all(target_os = "windows", feature = "runtime-d3d9"))]
            LibrashaderError::D3D9FilterError(e) => classify_d3d9(e),
            #[cfg(feature = "runtime-vulkan")]
            LibrashaderError::VulkanFilterError(e) => classify_vk(e),
            #[cfg(all(target_vendor = "apple", feature = "runtime-metal"))]
            LibrashaderError::MetalFilterError(e) => classify_mtl(e),
            _ => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &self.to_string()),
        }
    }
}

/// The source category of a librashader error. Identifies which subsystem produced
/// the innermost cause for an error returned via the C API.
#[repr(u32)]
pub enum LIBRA_ERROR_SOURCE {
    /// Unclassified runtime error. The `message` field holds the Display string.
    RUNTIME = 0,
    /// An error from the preset parser.
    PRESET_PARSE = 1,
    /// An error from the shader preprocessor.
    PREPROCESSOR = 2,
    /// An error from glslang during shader compilation.
    GLSLANG_ERROR = 3,
    /// An error from SPIRV-Cross. `code` carries an `spvc_result` value
    /// (see [`LIBRA_SPVC_RESULT`]) for the four canonical SPIRV-Cross error codes;
    /// other variants return 0.
    SPVC_RESULT = 4,
    /// An error from naga during shader compilation or reflection.
    NAGA_ERROR = 5,
    /// A raw `VkResult` from a Vulkan runtime call; the integer is in `code`.
    VK_RESULT = 6,
    /// An OpenGL runtime error. `code` carries a GL error code where one is available.
    GL_ERROR = 7,
    /// An `HRESULT` from a Direct3D runtime call; the integer is in `code`.
    HRESULT = 8,
    /// An IO error. `location.filename` carries the offending path.
    IO_ERROR = 9,
}

/// Discriminant for the metadata union inside a [`libra_error_details_t`].
#[repr(u32)]
pub enum LIBRA_ERROR_META_KIND {
    /// No metadata; `meta` is uninitialized and must not be read.
    NONE = 0,
    /// `meta.message` holds an owned C string.
    MESSAGE = 1,
    /// `meta.location` holds a file/offset/row/col record.
    LOCATION = 2,
}

/// Canonical `spvc_result` codes from SPIRV-Cross's `spirv_cross_c.h`. When
/// [`libra_error_details_t::source`] is [`LIBRA_ERROR_SOURCE::SPVC_RESULT`],
/// `code` is one of these values for the four error kinds with a C equivalent,
/// or 0 for an extended Rust-only variant.
#[repr(i32)]
pub enum LIBRA_SPVC_RESULT {
    /// `SPVC_ERROR_INVALID_SPIRV`.
    INVALID_SPIRV = -1,
    /// `SPVC_ERROR_UNSUPPORTED_SPIRV`.
    UNSUPPORTED_SPIRV = -2,
    /// `SPVC_ERROR_OUT_OF_MEMORY`.
    OUT_OF_MEMORY = -3,
    /// `SPVC_ERROR_INVALID_ARGUMENT`.
    INVALID_ARGUMENT = -4,
}

/// A file location reference inside a [`libra_error_details_t`].
///
/// All `*const c_char` fields are owned by the details struct and must be released
/// via `libra_error_free_details`.
#[repr(C)]
pub struct libra_file_location_error_t {
    /// Path of the file the error refers to. May be null when no path is known.
    pub filename: *const c_char,
    /// Byte offset into the file. Zero when the inner variant doesn't carry one.
    pub offset: usize,
    /// 1-based row (line) where the error occurred. Zero when unknown.
    pub row: u32,
    /// 1-based column where the error occurred. Zero when unknown.
    pub col: usize,
}

/// Metadata attached to a [`libra_error_details_t`]. Must be interpreted according
/// to the value of [`libra_error_details_t::meta_kind`].
#[repr(C)]
pub union libra_error_meta_t {
    /// Owned C string when `meta_kind` is [`LIBRA_ERROR_META_KIND::MESSAGE`].
    pub message: *const c_char,
    /// File location when `meta_kind` is [`LIBRA_ERROR_META_KIND::LOCATION`].
    pub location: ManuallyDrop<libra_file_location_error_t>,
}

/// Structured details for an error returned by the librashader C API.
///
/// Obtain via `libra_error_get_details`. Strings inside (`meta.message` and
/// `meta.location.filename`) are owned copies independent of the originating
/// `libra_error_t`; release them by calling `libra_error_free_details`.
#[repr(C)]
pub struct libra_error_details_t {
    /// Category of the innermost cause.
    pub source: LIBRA_ERROR_SOURCE,
    /// A subsystem-specific numeric code (HRESULT, VkResult, spvc_result, etc.).
    /// Zero when the inner variant doesn't carry one.
    pub code: i32,
    /// Discriminant for `meta`.
    pub meta_kind: LIBRA_ERROR_META_KIND,
    /// Optional metadata (message string or file location).
    pub meta: libra_error_meta_t,
}

/// Function pointer definition for libra_error_get_details
pub type PFN_libra_error_get_details =
    extern "C" fn(error: libra_error_t, out: *mut MaybeUninit<libra_error_details_t>) -> i32;

#[no_mangle]
/// Inspect an error and fill `out` with structured details about its innermost cause.
///
/// On success, `out` is populated with a [`libra_error_details_t`]. Any string fields
/// inside (`meta.message` and `meta.location.filename`) are heap-allocated owned copies
/// — release them by calling `libra_error_free_details` once the caller is done reading.
///
/// If `error` is null, this function does nothing and returns 1. Otherwise returns 0.
///
/// ## Safety
///   - `error` must be a valid and initialized instance of `libra_error_t`.
///   - `out` must be a non-null, properly aligned pointer to a `libra_error_details_t`.
pub unsafe extern "C" fn libra_error_get_details(
    error: libra_error_t,
    out: *mut MaybeUninit<libra_error_details_t>,
) -> i32 {
    let Some(error) = error else { return 1 };
    if out.is_null() {
        return 1;
    }

    unsafe {
        let details = error.as_ref().classify();
        out.write(MaybeUninit::new(details));
    }
    0
}

/// Function pointer definition for libra_error_free_details
pub type PFN_libra_error_free_details =
    extern "C" fn(details: *mut libra_error_details_t) -> i32;

#[no_mangle]
/// Release any heap allocations owned by a [`libra_error_details_t`] previously
/// populated by `libra_error_get_details`.
///
/// Reads `meta_kind` to determine which (if any) interior strings need to be freed,
/// then zeros the meta field. The caller still owns the `libra_error_details_t`
/// struct itself; this function only releases its interior allocations.
///
/// If `details` is null this function does nothing and returns 1. Otherwise returns 0.
///
/// ## Safety
///   - `details` must be null or point to a `libra_error_details_t` whose interior
///     strings were allocated by `libra_error_get_details`. Passing a struct from any
///     other source, or freeing twice, is undefined behaviour.
pub unsafe extern "C" fn libra_error_free_details(details: *mut libra_error_details_t) -> i32 {
    if details.is_null() {
        return 1;
    }

    unsafe {
        let d = &mut *details;
        match d.meta_kind {
            LIBRA_ERROR_META_KIND::NONE => {}
            LIBRA_ERROR_META_KIND::MESSAGE => {
                let ptr = d.meta.message as *mut c_char;
                if !ptr.is_null() {
                    drop(CString::from_raw(ptr));
                }
            }
            LIBRA_ERROR_META_KIND::LOCATION => {
                let filename = d.meta.location.filename as *mut c_char;
                if !filename.is_null() {
                    drop(CString::from_raw(filename));
                }
            }
        }
        d.meta_kind = LIBRA_ERROR_META_KIND::NONE;
        d.meta = libra_error_meta_t {
            message: std::ptr::null(),
        };
    }
    0
}

fn alloc_cstring(s: &str) -> *const c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw() as *const c_char,
        Err(_) => std::ptr::null(),
    }
}

fn alloc_path(p: &std::path::Path) -> *const c_char {
    alloc_cstring(&p.to_string_lossy())
}

fn details_none(source: LIBRA_ERROR_SOURCE, code: i32) -> libra_error_details_t {
    libra_error_details_t {
        source,
        code,
        meta_kind: LIBRA_ERROR_META_KIND::NONE,
        meta: libra_error_meta_t {
            message: std::ptr::null(),
        },
    }
}

fn details_message(
    source: LIBRA_ERROR_SOURCE,
    code: i32,
    message: &str,
) -> libra_error_details_t {
    libra_error_details_t {
        source,
        code,
        meta_kind: LIBRA_ERROR_META_KIND::MESSAGE,
        meta: libra_error_meta_t {
            message: alloc_cstring(message),
        },
    }
}

fn details_location(
    source: LIBRA_ERROR_SOURCE,
    code: i32,
    filename: Option<&std::path::Path>,
    offset: usize,
    row: u32,
    col: usize,
) -> libra_error_details_t {
    libra_error_details_t {
        source,
        code,
        meta_kind: LIBRA_ERROR_META_KIND::LOCATION,
        meta: libra_error_meta_t {
            location: ManuallyDrop::new(libra_file_location_error_t {
                filename: filename.map(alloc_path).unwrap_or(std::ptr::null()),
                offset,
                row,
                col,
            }),
        },
    }
}

fn parse_error_kind_code(kind: &librashader::presets::ParseErrorKind) -> i32 {
    use librashader::presets::ParseErrorKind as K;
    match kind {
        K::Index(_) => 1,
        K::Int => 2,
        K::UnsignedInt => 3,
        K::Float => 4,
        K::Bool => 5,
    }
}

fn unwrap_preset_infile(
    e: &librashader::presets::ParsePresetError,
) -> (Option<&std::path::Path>, &librashader::presets::ParsePresetError) {
    use librashader::presets::ParsePresetError as E;
    let mut path: Option<&std::path::Path> = None;
    let mut current = e;
    while let E::InFile { path: p, source } = current {
        path = Some(p.as_path());
        current = source.as_ref();
    }
    (path, current)
}

fn classify_preset(e: &librashader::presets::ParsePresetError) -> libra_error_details_t {
    use librashader::presets::ParsePresetError as E;
    let (path, inner) = unwrap_preset_infile(e);
    match inner {
        E::LexerError { offset, row, col } => details_location(
            LIBRA_ERROR_SOURCE::PRESET_PARSE,
            0,
            path,
            *offset,
            *row,
            *col,
        ),
        E::ParserError {
            offset,
            row,
            col,
            kind,
        } => details_location(
            LIBRA_ERROR_SOURCE::PRESET_PARSE,
            parse_error_kind_code(kind),
            path,
            *offset,
            *row,
            *col,
        ),
        E::IOError(p, _) => details_location(LIBRA_ERROR_SOURCE::IO_ERROR, 0, Some(p), 0, 0, 0),
        _ => details_message(LIBRA_ERROR_SOURCE::PRESET_PARSE, 0, &inner.to_string()),
    }
}

fn unwrap_preprocess_infile(
    e: &librashader::preprocess::PreprocessError,
) -> (
    Option<&std::path::Path>,
    &librashader::preprocess::PreprocessError,
) {
    use librashader::preprocess::PreprocessError as E;
    let mut path: Option<&std::path::Path> = None;
    let mut current = e;
    while let E::InFile { path: p, source } = current {
        path = Some(p.as_path());
        current = source.as_ref();
    }
    (path, current)
}

fn classify_preprocess(e: &librashader::preprocess::PreprocessError) -> libra_error_details_t {
    use librashader::preprocess::PreprocessError as E;
    let (path, inner) = unwrap_preprocess_infile(e);
    match inner {
        E::IOError(p, _) | E::EncodingError(p) => {
            details_location(LIBRA_ERROR_SOURCE::IO_ERROR, 0, Some(p), 0, 0, 0)
        }
        E::UnexpectedEol(line) => details_location(
            LIBRA_ERROR_SOURCE::PREPROCESSOR,
            0,
            path,
            0,
            *line as u32,
            0,
        ),
        E::MissingVersionHeader
        | E::UnexpectedEof
        | E::PragmaParseError(_)
        | E::DuplicatePragmaError(_)
        | E::UnknownImageFormat
        | E::InvalidStage => details_location(
            LIBRA_ERROR_SOURCE::PREPROCESSOR,
            0,
            path,
            0,
            0,
            0,
        ),
        _ => details_message(LIBRA_ERROR_SOURCE::PREPROCESSOR, 0, &inner.to_string()),
    }
}

fn classify_compile(e: &librashader::reflect::ShaderCompileError) -> libra_error_details_t {
    use librashader::reflect::ShaderCompileError as E;
    match e {
        E::GlslangError(_) | E::CompilerInitError => {
            details_message(LIBRA_ERROR_SOURCE::GLSLANG_ERROR, 0, &e.to_string())
        }
        E::SpirvCrossCompileError(_) => {
            details_message(LIBRA_ERROR_SOURCE::SPVC_RESULT, 0, &e.to_string())
        }
        E::NagaCompileError(_)
        | E::NagaWgslError(_)
        | E::NagaSpvError(_)
        | E::NagaMslError(_)
        | E::NagaValidationError(_) => {
            details_message(LIBRA_ERROR_SOURCE::NAGA_ERROR, 0, &e.to_string())
        }
        _ => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
    }
}

fn classify_reflect(e: &librashader::reflect::ShaderReflectError) -> libra_error_details_t {
    use librashader::reflect::ShaderReflectError as E;
    match e {
        E::SpirvCrossError(_) => {
            details_message(LIBRA_ERROR_SOURCE::SPVC_RESULT, 0, &e.to_string())
        }
        E::NagaInputError(_) | E::NagaReflectError(_) => {
            details_message(LIBRA_ERROR_SOURCE::NAGA_ERROR, 0, &e.to_string())
        }
        _ => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
    }
}

#[cfg(feature = "runtime-opengl")]
fn classify_gl(e: &librashader::runtime::gl::error::FilterChainError) -> libra_error_details_t {
    use librashader::runtime::gl::error::FilterChainError as E;
    match e {
        E::FramebufferInit(c) => details_none(LIBRA_ERROR_SOURCE::GL_ERROR, *c as i32),
        E::ShaderPresetError(p) => classify_preset(p),
        E::ShaderPreprocessError(p) => classify_preprocess(p),
        E::ShaderCompileError(c) => classify_compile(c),
        E::ShaderReflectError(r) => classify_reflect(r),
        E::SpirvCrossReflectError(_) => {
            details_message(LIBRA_ERROR_SOURCE::SPVC_RESULT, 0, &e.to_string())
        }
        E::LutLoadError(_) => details_message(LIBRA_ERROR_SOURCE::IO_ERROR, 0, &e.to_string()),
        E::GLLinkError(_) | E::GlCompileError(_) | E::GlProgramError(_) => {
            details_message(LIBRA_ERROR_SOURCE::GL_ERROR, 0, &e.to_string())
        }
        E::GLLoadError
        | E::GlSamplerError
        | E::GlInvalidFramebuffer
        | E::GlError(_) => details_message(LIBRA_ERROR_SOURCE::GL_ERROR, 0, &e.to_string()),
        _ => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
    }
}

#[cfg(all(target_os = "windows", feature = "runtime-d3d11"))]
fn classify_d3d11(
    e: &librashader::runtime::d3d11::error::FilterChainError,
) -> libra_error_details_t {
    use librashader::runtime::d3d11::error::FilterChainError as E;
    match e {
        E::Direct3DError(err) => {
            details_message(LIBRA_ERROR_SOURCE::HRESULT, err.code().0, &err.to_string())
        }
        E::D3DCompileError(err, log) => {
            details_message(LIBRA_ERROR_SOURCE::HRESULT, err.code().0, log)
        }
        E::Direct3DOperationError(s) => {
            details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, s)
        }
        E::ShaderPresetError(p) => classify_preset(p),
        E::ShaderPreprocessError(p) => classify_preprocess(p),
        E::ShaderCompileError(c) => classify_compile(c),
        E::ShaderReflectError(r) => classify_reflect(r),
        E::LutLoadError(_) => details_message(LIBRA_ERROR_SOURCE::IO_ERROR, 0, &e.to_string()),
        _ => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
    }
}

#[cfg(all(target_os = "windows", feature = "runtime-d3d12"))]
fn classify_d3d12(
    e: &librashader::runtime::d3d12::error::FilterChainError,
) -> libra_error_details_t {
    use librashader::runtime::d3d12::error::FilterChainError as E;
    match e {
        E::Direct3DError(err) => {
            details_message(LIBRA_ERROR_SOURCE::HRESULT, err.code().0, &err.to_string())
        }
        E::Direct3DOperationError(s) => {
            details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, s)
        }
        E::ShaderPresetError(p) => classify_preset(p),
        E::ShaderPreprocessError(p) => classify_preprocess(p),
        E::ShaderCompileError(c) => classify_compile(c),
        E::ShaderReflectError(r) => classify_reflect(r),
        E::LutLoadError(_) => details_message(LIBRA_ERROR_SOURCE::IO_ERROR, 0, &e.to_string()),
        E::HeapError(_)
        | E::AllocationError(_)
        | E::InvalidDimensionError(_) => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
        _ => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
    }
}

#[cfg(all(target_os = "windows", feature = "runtime-d3d9"))]
fn classify_d3d9(
    e: &librashader::runtime::d3d9::error::FilterChainError,
) -> libra_error_details_t {
    use librashader::runtime::d3d9::error::FilterChainError as E;
    match e {
        E::Direct3DError(err) => {
            details_message(LIBRA_ERROR_SOURCE::HRESULT, err.code().0, &err.to_string())
        }
        E::Direct3DOperationError(s) => {
            details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, s)
        }
        E::ShaderPresetError(p) => classify_preset(p),
        E::ShaderPreprocessError(p) => classify_preprocess(p),
        E::ShaderCompileError(c) => classify_compile(c),
        E::ShaderReflectError(r) => classify_reflect(r),
        E::LutLoadError(_) => details_message(LIBRA_ERROR_SOURCE::IO_ERROR, 0, &e.to_string()),
        _ => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
    }
}

#[cfg(feature = "runtime-vulkan")]
fn classify_vk(e: &librashader::runtime::vk::error::FilterChainError) -> libra_error_details_t {
    use librashader::runtime::vk::error::FilterChainError as E;
    match e {
        E::HandleIsNull => details_none(LIBRA_ERROR_SOURCE::RUNTIME, 0),
        E::VulkanResult(r) => details_none(LIBRA_ERROR_SOURCE::VK_RESULT, r.as_raw()),
        E::VulkanMemoryError(_)
        | E::AllocationError(_)
        | E::AllocationDoesNotExist => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
        E::ShaderPresetError(p) => classify_preset(p),
        E::ShaderPreprocessError(p) => classify_preprocess(p),
        E::ShaderCompileError(c) => classify_compile(c),
        E::ShaderReflectError(r) => classify_reflect(r),
        E::LutLoadError(_) => details_message(LIBRA_ERROR_SOURCE::IO_ERROR, 0, &e.to_string()),
        _ => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
    }
}

#[cfg(all(target_vendor = "apple", feature = "runtime-metal"))]
fn classify_mtl(e: &librashader::runtime::mtl::error::FilterChainError) -> libra_error_details_t {
    use librashader::runtime::mtl::error::FilterChainError as E;
    match e {
        E::ShaderPresetError(p) => classify_preset(p),
        E::ShaderPreprocessError(p) => classify_preprocess(p),
        E::ShaderCompileError(c) => classify_compile(c),
        E::ShaderReflectError(r) => classify_reflect(r),
        E::LutLoadError(_) => details_message(LIBRA_ERROR_SOURCE::IO_ERROR, 0, &e.to_string()),
        _ => details_message(LIBRA_ERROR_SOURCE::RUNTIME, 0, &e.to_string()),
    }
}

macro_rules! assert_non_null {
    (@EXPORT $value:ident) => {
        if $value.is_null() || !$crate::ffi::ptr_is_aligned($value) {
            return $crate::error::LibrashaderError::InvalidParameter(stringify!($value)).export();
        }
    };
    ($value:ident) => {
        if $value.is_null() || !$crate::ffi::ptr_is_aligned($value) {
            return Err($crate::error::LibrashaderError::InvalidParameter(
                stringify!($value),
            ));
        }
    };
}

macro_rules! assert_some_ptr {
    ($value:ident) => {
        if $value.is_none() {
            return Err($crate::error::LibrashaderError::InvalidParameter(
                stringify!($value),
            ));
        }

        let $value = unsafe { $value.as_ref().unwrap_unchecked().as_ref() };
    };
    (mut $value:ident) => {
        if $value.is_none() {
            return Err($crate::error::LibrashaderError::InvalidParameter(
                stringify!($value),
            ));
        }

        let $value = unsafe { $value.as_mut().unwrap_unchecked().as_mut() };
    };
}

use crate::ctypes::libra_error_t;
pub(crate) use assert_non_null;

// pub(crate) use assert_some;
pub(crate) use assert_some_ptr;
