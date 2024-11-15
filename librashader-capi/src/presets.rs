//! librashader preset C API (`libra_preset_*`).
use crate::ctypes::{libra_preset_ctx_t, libra_shader_preset_t};
use crate::error::{assert_non_null, assert_some_ptr, LibrashaderError};
use crate::ffi::extern_fn;
use crate::LIBRASHADER_API_VERSION;
use librashader::presets::{ShaderFeatures, ShaderPreset, WildcardContext};
use std::ffi::{c_char, CStr, CString};
use std::mem::MaybeUninit;
use std::ptr::{addr_of_mut, NonNull};

const _: () = crate::assert_thread_safe::<ShaderPreset>();

/// A list of preset parameters.
#[repr(C)]
pub struct libra_preset_param_list_t {
    /// A pointer to the parameter
    pub parameters: *const libra_preset_param_t,
    /// The number of parameters in the list. This field
    /// is readonly, and changing it will lead to undefined
    /// behaviour on free.
    pub length: u64,
}

/// A preset parameter.
#[repr(C)]
pub struct libra_preset_param_t {
    /// The name of the parameter
    pub name: *const c_char,
    /// The description of the parameter.
    pub description: *const c_char,
    /// The initial value the parameter is set to.
    pub initial: f32,
    /// The minimum value that the parameter can be set to.
    pub minimum: f32,
    /// The maximum value that the parameter can be set to.
    pub maximum: f32,
    /// The step by which this parameter can be incremented or decremented.
    pub step: f32,
}

/// Options struct for loading shader presets.
///
/// Using this struct with `libra_preset_create_with_options` is the only way to
/// enable extended shader preset features.
#[repr(C)]
pub struct libra_preset_opt_t {
    /// The librashader API version.
    pub version: LIBRASHADER_API_VERSION,
    /// Enables `_HAS_ORIGINALASPECT_UNIFORMS` behaviour.
    ///
    /// If this is true, then `frame_options.aspect_ratio` must be set for correct behaviour of shaders.
    ///
    /// This is only supported on API 2 and above, otherwise this has no effect.
    pub original_aspect_uniforms: bool,
    /// Enables `_HAS_FRAMETIME_UNIFORMS` behaviour.
    ///
    /// If this is true, then `frame_options.frames_per_second` and `frame_options.frametime_delta`
    /// must be set for correct behaviour of shaders.
    ///
    /// This is only supported on API 2 and above, otherwise this has no effect.
    pub frametime_uniforms: bool,
}

extern_fn! {
    /// Load a preset.
    ///
    /// This function is deprecated, and `libra_preset_create_with_options` should be used instead.
    /// ## Safety
    ///  - `filename` must be either null or a valid, aligned pointer to a string path to the shader preset.
    ///  - `out` must be either null, or an aligned pointer to an uninitialized or invalid `libra_shader_preset_t`.
    /// ## Returns
    ///  - If any parameters are null, `out` is unchanged, and this function returns `LIBRA_ERR_INVALID_PARAMETER`.
    fn libra_preset_create(
        filename: *const c_char,
        out: *mut MaybeUninit<libra_shader_preset_t>
    ) {
        assert_non_null!(filename);
        assert_non_null!(out);

        let filename = unsafe { CStr::from_ptr(filename) };
        let filename = filename.to_str()?;

        let preset = ShaderPreset::try_parse(filename, ShaderFeatures::NONE)?;
        unsafe {
            out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(
                preset,
            )))))
        }
    }
}

extern_fn! {
    /// Load a preset with the given wildcard context.
    ///
    /// The wildcard context is immediately invalidated and must be recreated after
    /// the preset is created.
    ///
    /// Path information variables `PRESET_DIR` and `PRESET` will automatically be filled in.
    ///
    /// This function is deprecated, and `libra_preset_create_with_options` should be used instead.
    /// ## Safety
    ///  - `filename` must be either null or a valid, aligned pointer to a string path to the shader preset.
    ///  - `context` must be either null or a valid, aligned pointer to a initialized `libra_preset_ctx_t`.
    ///  - `context` is  invalidated after this function returns.
    ///  - `out` must be either null, or an aligned pointer to an uninitialized or invalid `libra_shader_preset_t`.
    /// ## Returns
    ///  - If any parameters are null, `out` is unchanged, and this function returns `LIBRA_ERR_INVALID_PARAMETER`.
    fn libra_preset_create_with_context(
        filename: *const c_char,
        context: *mut libra_preset_ctx_t,
        out: *mut MaybeUninit<libra_shader_preset_t>
    ) {
        assert_non_null!(filename);
        assert_non_null!(context);
        assert_non_null!(out);

        let filename = unsafe { CStr::from_ptr(filename) };
        let filename = filename.to_str()?;

        let mut context = unsafe {
            let context_ptr = &mut *context;
            let context = context_ptr.take();
            Box::from_raw(context.unwrap().as_ptr())
        };

        context.add_path_defaults(filename);

        let preset = ShaderPreset::try_parse_with_context(filename, ShaderFeatures::NONE, *context)?;
        unsafe {
            out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(
                preset,
            )))))
        }
    }
}

extern_fn! {
    /// Load a preset with optional options and an optional context.
    ///
    /// Both `context` and `options` may be null.
    ///
    /// If `context` is null, then a default context will be provided, and this function will not return `LIBRA_ERR_INVALID_PARAMETER`.
    /// If `options` is null, then default options will be chosen.
    ///
    /// If `context` is provided, it is immediately invalidated and must be recreated after
    /// the preset is created.
    ///
    /// ## Safety
    ///  - `filename` must be either null or a valid, aligned pointer to a string path to the shader preset.
    ///  - `context` must be either null or a valid, aligned pointer to an initialized `libra_preset_ctx_t`.
    ///  - `options` must be either null, or a valid, aligned pointer to a `libra_shader_opt_t`.
    ///    `LIBRASHADER_API_VERSION` should be set to `LIBRASHADER_CURRENT_VERSION`.
    ///  - `out` must be either null, or an aligned pointer to an uninitialized or invalid `libra_shader_preset_t`.
    ///
    /// ## Returns
    ///  - If `out` or `filename` is null, `out` is unchanged, and this function returns `LIBRA_ERR_INVALID_PARAMETER`.
    fn libra_preset_create_with_options(
        filename: *const c_char,
        context: *mut libra_preset_ctx_t,
        options: *mut MaybeUninit<libra_preset_opt_t>,
        out: *mut MaybeUninit<libra_shader_preset_t>
    ) {
        assert_non_null!(filename);
        assert_non_null!(out);

        let filename = unsafe { CStr::from_ptr(filename) };
        let filename = filename.to_str()?;

        // This control flow is like this because the wrapper makes it hard to return early..
        if options.is_null() {
            let preset = ShaderPreset::try_parse(filename, ShaderFeatures::NONE)?;
            unsafe {
                out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(
                    preset,
                )))))
            }
        } else {
            // SAFETY: options is not null
            let mut options = unsafe { options.read() };
            let opt_ptr = options.as_mut_ptr();

            let api_version = unsafe { addr_of_mut!((*opt_ptr).version).read() };

            let mut context = if context.is_null() {
                Box::new(WildcardContext::new())
            } else {
                unsafe {
                    let context_ptr = &mut *context;
                    let context = context_ptr.take();
                    Box::from_raw(context.unwrap().as_ptr())
                }
            };

            context.add_path_defaults(filename);

            let mut flags = ShaderFeatures::NONE;

            // Original Aspect and Frametime Uniforms are an API 2 feature.
            if api_version >= 2 {
                let original_aspect_uniforms = unsafe { addr_of_mut!((*opt_ptr).original_aspect_uniforms).read() };
                let frametime_uniforms = unsafe { addr_of_mut!((*opt_ptr).frametime_uniforms).read() };

                if original_aspect_uniforms {
                    flags |= ShaderFeatures::ORIGINAL_ASPECT_UNIFORMS;
                }

                if frametime_uniforms {
                    flags |= ShaderFeatures::FRAMETIME_UNIFORMS;
                }
            }

            let preset = ShaderPreset::try_parse(filename, flags)?;
            unsafe {
                out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(
                    preset,
                )))))
            }
        }
    }
}

extern_fn! {
    /// Free the preset.
    ///
    /// If `preset` is null, this function does nothing. The resulting value in `preset` then becomes
    /// null.
    ///
    /// ## Safety
    /// - `preset` must be a valid and aligned pointer to a `libra_shader_preset_t`.
    fn libra_preset_free(preset: *mut libra_shader_preset_t) {
        assert_non_null!(preset);
        unsafe {
            let preset_ptr = &mut *preset;
            let preset = preset_ptr.take();
            drop(Box::from_raw(preset.unwrap().as_ptr()));
        }
    }
}

extern_fn! {
    /// Set the value of the parameter in the preset.
    ///
    /// ## Safety
    /// - `preset` must be null or a valid and aligned pointer to a `libra_shader_preset_t`.
    /// - `name` must be null or a valid and aligned pointer to a string.
    fn libra_preset_set_param(
        preset: *mut libra_shader_preset_t,
        name: *const c_char,
        value: f32
    ) |name|; mut |preset| {
        let name = unsafe {
            CStr::from_ptr(name)
        };

        let name = name.to_str()?;
        assert_some_ptr!(mut preset);

        if let Some(param) = preset.parameters.iter_mut().find(|c| c.name == name) {
            param.value = value
        }
    }
}

extern_fn! {
    /// Get the value of the parameter as set in the preset.
    ///
    /// ## Safety
    /// - `preset` must be null or a valid and aligned pointer to a shader preset.
    /// - `name` must be null or a valid and aligned pointer to a string.
    /// - `value` may be a pointer to a uninitialized `float`.
    fn libra_preset_get_param(
        preset: *const libra_shader_preset_t,
        name: *const c_char,
        value: *mut MaybeUninit<f32>
    ) |name, preset| {
        let name = unsafe { CStr::from_ptr(name) };
        let name = name.to_str()?;
        assert_some_ptr!(preset);
        assert_non_null!(value);

        if let Some(param) = preset.parameters.iter().find(|c| c.name == name) {
            unsafe { value.write(MaybeUninit::new(param.value)) }
        }
    }
}

extern_fn! {
    /// Pretty print the shader preset.
    ///
    /// ## Safety
    /// - `preset` must be null or a valid and aligned pointer to a `libra_shader_preset_t`.
    fn libra_preset_print(preset: *mut libra_shader_preset_t) |preset| {
        assert_some_ptr!(preset);
        println!("{preset:#?}");
    }
}

extern_fn! {
    /// Get a list of runtime parameters.
    ///
    /// ## Safety
    /// - `preset` must be null or a valid and aligned pointer to a `libra_shader_preset_t`.
    /// - `out` must be an aligned pointer to a `libra_preset_parameter_list_t`.
    /// - The output struct should be treated as immutable. Mutating any struct fields
    ///   in the returned struct may at best cause memory leaks, and at worse
    ///   cause undefined behaviour when later freed.
    /// - It is safe to call `libra_preset_get_runtime_params` multiple times, however
    ///   the output struct must only be freed once per call.
    fn libra_preset_get_runtime_params(
        preset: *const libra_shader_preset_t,
        out: *mut MaybeUninit<libra_preset_param_list_t>
    ) |preset| {
        assert_some_ptr!(preset);
        assert_non_null!(out);

        let iter = librashader::presets::get_parameter_meta(preset)?;
        let mut values = Vec::new();
        for param in iter {
            let name = CString::new(param.id.to_string())
            .map_err(|err| LibrashaderError::UnknownError(Box::new(err)))?;
            let description = CString::new(param.description)
            .map_err(|err| LibrashaderError::UnknownError(Box::new(err)))?;
            values.push(libra_preset_param_t {
                name: name.into_raw().cast_const(),
                description: description.into_raw().cast_const(),
                initial: param.initial,
                minimum: param.minimum,
                maximum: param.maximum,
                step: param.step
            })
        }

        let values = values.into_boxed_slice();
        let (parts, len) = crate::ffi::boxed_slice_into_raw_parts(values);

        unsafe {
            out.write(MaybeUninit::new(libra_preset_param_list_t {
                parameters: parts,
                length: len as u64,
            }));
        }
    }
}

extern_fn! {
    /// Free the runtime parameters.
    ///
    /// Unlike the other `free` functions provided by librashader,
    /// `libra_preset_free_runtime_params` takes the struct directly.
    /// The caller must take care to maintain the lifetime of any pointers
    /// contained within the input `libra_preset_param_list_t`.
    ///
    /// ## Safety
    /// - Any pointers rooted at `parameters` becomes invalid after this function returns,
    ///   including any strings accessible via the input `libra_preset_param_list_t`.
    ///   The caller must ensure that there are no live pointers, aliased or unaliased,
    ///   to data accessible via the input `libra_preset_param_list_t`.
    ///
    /// - Accessing any data pointed to via the input `libra_preset_param_list_t` after it
    ///   has been freed is a use-after-free and is immediate undefined behaviour.
    ///
    /// - If any struct fields of the input `libra_preset_param_list_t` was modified from
    ///   their values given after `libra_preset_get_runtime_params`, this may result
    ///   in undefined behaviour.
    fn libra_preset_free_runtime_params(preset: libra_preset_param_list_t) {
        unsafe {
            let values =
                    crate::ffi::boxed_slice_from_raw_parts(preset.parameters.cast_mut(),
                preset.length as usize).into_vec();

            for value in values {
                let name = CString::from_raw(value.name.cast_mut());
                let description = CString::from_raw(value.description.cast_mut());

                drop(name);
                drop(description)
            }
        }
    }
}
