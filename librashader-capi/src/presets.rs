//! The librashader preset C API (`libra_preset_*`).
use std::ffi::{c_char, CStr, CString};
use std::mem::{MaybeUninit};
use librashader::presets::ShaderPreset;
use crate::ffi::ffi_body;
use crate::ctypes::{libra_error_t, libra_shader_preset_t};
use crate::error::{assert_non_null, assert_some, assert_some_ptr, LibrashaderError};
use std::ptr::NonNull;

pub type PFN_lbr_preset_create = unsafe extern "C" fn (*const c_char, *mut MaybeUninit<libra_shader_preset_t>) -> libra_error_t;

/// Load a preset.
///
/// ## Safety
///  - `filename` must be either null or a valid, aligned pointer to a string path to the shader preset.
///  - `out` must be either null, or an aligned pointer to an uninitialized or invalid `libra_shader_preset_t`.
/// ## Returns
///  - If any parameters are null, `out` is unchanged, and this function returns `LIBRA_ERR_INVALID_PARAMETER`.
#[no_mangle]
pub unsafe extern "C" fn libra_preset_create(filename: *const c_char, out: *mut MaybeUninit<libra_shader_preset_t>) -> libra_error_t {
    ffi_body!({
        assert_non_null!(filename);
        assert_non_null!(out);

        let filename = unsafe {
            CStr::from_ptr(filename)
        };

        let filename = filename.to_str()?;

        println!("loading {filename}");
        let preset = ShaderPreset::try_parse(filename)?;

        unsafe {
            out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(preset)))))
        }
    })
}

pub type PFN_lbr_preset_free = unsafe extern "C" fn (*mut libra_shader_preset_t) -> libra_error_t;

/// Free the preset.
///
/// If `preset` is null, this function does nothing. The resulting value in `preset` then becomes
/// null.
///
/// ## Safety
/// - `preset` must be a valid and aligned pointer to a shader preset.
#[no_mangle]
pub unsafe extern "C" fn libra_preset_free(preset: *mut libra_shader_preset_t) -> libra_error_t {
    ffi_body!({
        assert_non_null!(preset);
        unsafe {
            let preset_ptr = &mut *preset;
            let preset = preset_ptr.take();
            drop(Box::from_raw(preset.unwrap().as_ptr()));
        }
    })
}

pub type PFN_lbr_preset_set_param = unsafe extern "C" fn (*mut libra_shader_preset_t, *const c_char, f32) -> libra_error_t;
/// Set the value of the parameter in the preset.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
/// - `name` must be null or a valid and aligned pointer to a string.
#[no_mangle]
pub unsafe extern "C" fn libra_preset_set_param(preset: *mut libra_shader_preset_t,
                                                name: *const c_char, value: f32) -> libra_error_t {
    ffi_body!(|name|; mut |preset| {
        let name = unsafe {
            CStr::from_ptr(name)
        };

        let name = name.to_str()?;
        assert_some_ptr!(mut preset);

        if let Some(param) = preset.parameters.iter_mut().find(|c| c.name == name) {
            param.value = value
        }
    })
}

pub type PFN_lbr_preset_get_param = unsafe extern "C" fn (*mut libra_shader_preset_t, *const c_char, *mut MaybeUninit<f32>) -> libra_error_t;

/// Get the value of the parameter as set in the preset.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
/// - `name` must be null or a valid and aligned pointer to a string.
/// - `value` may be a pointer to a uninitialized `float`.
#[no_mangle]
pub unsafe extern "C" fn libra_preset_get_param(preset: *mut libra_shader_preset_t,
                                                name: *const c_char, value: *mut MaybeUninit<f32>) -> libra_error_t {
    ffi_body!(|name, preset | {
        let name = unsafe {
            CStr::from_ptr(name)
        };

        let name = name.to_str()?;
        assert_some_ptr!(preset);

        assert_non_null!(value);

        if let Some(param) = preset.parameters.iter().find(|c| c.name == name) {
            unsafe {
                value.write(MaybeUninit::new(param.value))
            }
        }
    })
}

pub type PFN_lbr_preset_print = unsafe extern "C" fn (*mut libra_shader_preset_t) -> libra_error_t;

/// Pretty print the shader preset.
///
/// ## Safety
/// - `preset` must be null or a valid and aligned pointer to a shader preset.
#[no_mangle]
pub unsafe extern "C" fn libra_preset_print(preset: *mut libra_shader_preset_t) -> libra_error_t {
    ffi_body!(|preset| {
        assert_some_ptr!(preset);
        println!("{:#?}", preset);
    })
}


pub type PFN_lbr_preset_get_runtime_param_names = unsafe extern "C" fn (*mut libra_shader_preset_t, *mut MaybeUninit<f32>) -> libra_error_t;

/// Get a list of runtime parameter names.
///
/// The returned value can not currently be freed.
/// This function should be considered in progress. Its use is discouraged.
#[no_mangle]
pub unsafe extern "C" fn libra_preset_get_runtime_param_names(preset: *mut libra_shader_preset_t, mut value: MaybeUninit<*mut *const c_char>) -> libra_error_t {
    ffi_body!(|preset | {
        assert_some_ptr!(preset);

        let iter = librashader::presets::get_parameter_meta(preset)?;
        let mut c_strings = Vec::new();
        for param in iter {
            let c_string = CString::new(param.id).map_err(|err| LibrashaderError::UnknownError(Box::new(err)))?;
            c_strings.push(c_string.into_raw().cast_const());
        }

        let (parts, _len, _cap) = c_strings.into_raw_parts();
        value.write(parts);
    })
}
