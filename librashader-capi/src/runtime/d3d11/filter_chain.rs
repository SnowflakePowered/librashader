use std::ffi::CStr;
use crate::ctypes::{
    libra_d3d11_filter_chain_t, libra_shader_preset_t, libra_viewport_t,
};
use crate::error::{assert_non_null, assert_some_ptr, LibrashaderError};
use crate::ffi::extern_fn;
use librashader::runtime::d3d11::{D3D11InputView, D3D11OutputView};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::ffi::c_char;
use std::slice;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11RenderTargetView, ID3D11ShaderResourceView,
};

pub use librashader::runtime::d3d11::capi::options::FilterChainOptionsD3D11;
pub use librashader::runtime::d3d11::capi::options::FrameOptionsD3D11;

use librashader::runtime::{FilterChainParameters, Size, Viewport};

/// OpenGL parameters for the source image.
#[repr(C)]
pub struct libra_source_image_d3d11_t {
    /// A shader resource view into the source image
    pub handle: *const ID3D11ShaderResourceView,
    /// The width of the source image.
    pub width: u32,
    /// The height of the source image.
    pub height: u32,
}

impl TryFrom<libra_source_image_d3d11_t> for D3D11InputView {
    type Error = LibrashaderError;

    fn try_from(value: libra_source_image_d3d11_t) -> Result<Self, Self::Error> {
        let handle = value.handle;
        assert_non_null!(noexport handle);

        Ok(D3D11InputView {
            handle: unsafe { (&*handle).clone() },
            size: Size::new(value.width, value.height),
        })
    }
}

extern_fn! {
    /// Create the filter chain given the shader preset.
    ///
    /// The shader preset is immediately invalidated and must be recreated after
    /// the filter chain is created.
    ///
    /// ## Safety:
    /// - `preset` must be either null, or valid and aligned.
    /// - `options` must be either null, or valid and aligned.
    /// - `out` must be aligned, but may be null, invalid, or uninitialized.
    fn libra_d3d11_filter_chain_create(
        preset: *mut libra_shader_preset_t,
        options: *const FilterChainOptionsD3D11,
        device: *const ID3D11Device,
        out: *mut MaybeUninit<libra_d3d11_filter_chain_t>
    ) {
        assert_non_null!(preset);
        assert_non_null!(device);
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

        let chain = librashader::runtime::d3d11::capi::FilterChainD3D11::load_from_preset(
            unsafe { &*device },
            *preset,
            options,
        )?;

        unsafe {
            out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(
                chain,
            )))))
        }
    }
}

extern_fn! {
    /// Draw a frame with the given parameters for the given filter chain.
    ///
    /// ## Safety
    /// - `chain` may be null, invalid, but not uninitialized. If `chain` is null or invalid, this
    ///    function will return an error.
    /// - `mvp` may be null, or if it is not null, must be an aligned pointer to 16 consecutive `float`
    ///    values for the model view projection matrix.
    /// - `opt` may be null, or if it is not null, must be an aligned pointer to a valid `frame_gl_opt_t`
    ///    struct.
    fn libra_d3d11_filter_chain_frame(
        chain: *mut libra_d3d11_filter_chain_t,
        frame_count: usize,
        image: libra_source_image_d3d11_t,
        viewport: libra_viewport_t,
        out: *const ID3D11RenderTargetView,
        mvp: *const f32,
        opt: *const FrameOptionsD3D11
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        assert_non_null!(out);

        let mvp = if mvp.is_null() {
            None
        } else {
            Some(<&[f32; 16]>::try_from(unsafe { slice::from_raw_parts(mvp, 16) }).unwrap())
        };

        let opt = if opt.is_null() {
            None
        } else {
            Some(unsafe { opt.read() })
        };

        let viewport = Viewport {
            x: viewport.x,
            y: viewport.y,
            output: D3D11OutputView {
                size: Size::new(viewport.width, viewport.height),
                handle: unsafe { (&*out).clone() },
            },
            mvp,
        };

        let image = image.try_into()?;
        chain.frame(image, &viewport, frame_count, opt.as_ref())?;
    }
}

extern_fn! {
    /// Sets a parameter for the filter chain.
    ///
    /// If the parameter does not exist, returns an error.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d11_filter_chain_t`.
    /// - `param_name` must be either null or a null terminated string.
    fn libra_d3d11_filter_chain_set_param(
        chain: *mut libra_d3d11_filter_chain_t,
        param_name: *const c_char,
        value: f32
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        assert_non_null!(param_name);
        unsafe {
            let name = CStr::from_ptr(param_name);
            let name = name.to_str()?;

            if let None = chain.set_parameter(name, value) {
                return LibrashaderError::UnknownShaderParameter(param_name).export()
            }
        }
    }
}

extern_fn! {
    /// Gets a parameter for the filter chain.
    ///
    /// If the parameter does not exist, returns an error.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d11_filter_chain_t`.
    /// - `param_name` must be either null or a null terminated string.
    fn libra_d3d11_filter_chain_get_param(
        chain: *mut libra_d3d11_filter_chain_t,
        param_name: *const c_char,
        out: *mut MaybeUninit<f32>
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        assert_non_null!(param_name);
        unsafe {
            let name = CStr::from_ptr(param_name);
            let name = name.to_str()?;

            let Some(value) = chain.get_parameter(name) else {
                return LibrashaderError::UnknownShaderParameter(param_name).export()
            };

            out.write(MaybeUninit::new(value));
        }
    }
}

extern_fn! {
    /// Sets the number of active passes for this chain.
    ///
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d11_filter_chain_t`.
    fn libra_d3d11_filter_chain_set_active_pass_count(
        chain: *mut libra_d3d11_filter_chain_t,
        value: u32
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        unsafe {
            chain.set_enabled_pass_count(value as usize);
        }
    }
}

extern_fn! {
    /// Gets the number of active passes for this chain.
    ///
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d11_filter_chain_t`.
    fn libra_d3d11_filter_chain_get_active_pass_count(
        chain: *mut libra_d3d11_filter_chain_t,
        out: *mut MaybeUninit<u32>
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        unsafe {
            let value = chain.get_enabled_pass_count();
            out.write(MaybeUninit::new(value as u32))
        }
    }
}

extern_fn! {
    /// Free a D3D11 filter chain.
    ///
    /// The resulting value in `chain` then becomes null.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d11_filter_chain_t`.
    fn libra_d3d11_filter_chain_free(chain: *mut libra_d3d11_filter_chain_t) {
        assert_non_null!(chain);
        unsafe {
            let chain_ptr = &mut *chain;
            let chain = chain_ptr.take();
            drop(Box::from_raw(chain.unwrap().as_ptr()))
        };
    }
}
