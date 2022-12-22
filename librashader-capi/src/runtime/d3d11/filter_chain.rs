use crate::ctypes::{
    libra_d3d11_filter_chain_t, libra_error_t, libra_shader_preset_t, libra_viewport_t,
};
use crate::error::{assert_non_null, assert_some_ptr, LibrashaderError};
use crate::ffi::ffi_body;
use librashader::runtime::d3d11::{DxImageView, Viewport};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::slice;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11RenderTargetView, ID3D11ShaderResourceView,
};

pub use librashader::runtime::d3d11::options::FilterChainOptionsD3D11;
pub use librashader::runtime::d3d11::options::FrameOptionsD3D11;
use librashader::Size;

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

impl TryFrom<libra_source_image_d3d11_t> for DxImageView {
    type Error = LibrashaderError;

    fn try_from(value: libra_source_image_d3d11_t) -> Result<Self, Self::Error> {
        let handle = value.handle;
        assert_non_null!(noexport handle);

        Ok(DxImageView {
            handle: unsafe { (&*handle).clone() },
            size: Size::new(value.width, value.height),
        })
    }
}

pub type PFN_lbr_d3d11_filter_chain_create = unsafe extern "C" fn(
    preset: *mut libra_shader_preset_t,
    options: *const FilterChainOptionsD3D11,
    device: *const ID3D11Device,
    out: *mut MaybeUninit<libra_d3d11_filter_chain_t>,
) -> libra_error_t;
/// Create the filter chain given the shader preset.
///
/// The shader preset is immediately invalidated and must be recreated after
/// the filter chain is created.
///
/// ## Safety:
/// - `preset` must be either null, or valid and aligned.
/// - `options` must be either null, or valid and aligned.
/// - `out` must be aligned, but may be null, invalid, or uninitialized.
#[no_mangle]
pub unsafe extern "C" fn libra_d3d11_filter_chain_create(
    preset: *mut libra_shader_preset_t,
    options: *const FilterChainOptionsD3D11,
    device: *const ID3D11Device,
    out: *mut MaybeUninit<libra_d3d11_filter_chain_t>,
) -> libra_error_t {
    ffi_body!({
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

        let chain = librashader::runtime::d3d11::FilterChainD3D11::load_from_preset(
            unsafe { &*device },
            *preset,
            options,
        )?;

        unsafe {
            out.write(MaybeUninit::new(NonNull::new(Box::into_raw(Box::new(
                chain,
            )))))
        }
    })
}

pub type PFN_lbr_d3d11_filter_chain_frame = unsafe extern "C" fn(
    chain: *mut libra_d3d11_filter_chain_t,
    frame_count: usize,
    image: libra_source_image_d3d11_t,
    viewport: libra_viewport_t,
    out: *const ID3D11RenderTargetView,
    mvp: *const f32,
    opt: *const FrameOptionsD3D11,
) -> libra_error_t;

/// Draw a frame with the given parameters for the given filter chain.
///
/// ## Safety
/// - `chain` may be null, invalid, but not uninitialized. If `chain` is null or invalid, this
///    function will return an error.
/// - `mvp` may be null, or if it is not null, must be an aligned pointer to 16 consecutive `float`
///    values for the model view projection matrix.
/// - `opt` may be null, or if it is not null, must be an aligned pointer to a valid `frame_gl_opt_t`
///    struct.
#[no_mangle]
pub unsafe extern "C" fn libra_d3d11_filter_chain_frame(
    chain: *mut libra_d3d11_filter_chain_t,
    frame_count: usize,
    image: libra_source_image_d3d11_t,
    viewport: libra_viewport_t,
    out: *const ID3D11RenderTargetView,
    mvp: *const f32,
    opt: *const FrameOptionsD3D11,
) -> libra_error_t {
    ffi_body!(mut |chain| {
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
            size: Size::new(viewport.width, viewport.height),
            output: unsafe { (&*out).clone() },
            mvp,
        };

        let image = image.try_into()?;
        chain.frame(image, &viewport, frame_count, opt.as_ref())?;
    })
}

pub type PFN_lbr_d3d11_filter_chain_free =
    unsafe extern "C" fn(chain: *mut libra_d3d11_filter_chain_t) -> libra_error_t;
/// Free a D3D11 filter chain.
///
/// The resulting value in `chain` then becomes null.
/// ## Safety
/// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_d3d11_filter_chain_t`.
#[no_mangle]
pub unsafe extern "C" fn libra_d3d11_filter_chain_free(
    chain: *mut libra_d3d11_filter_chain_t,
) -> libra_error_t {
    ffi_body!({
        assert_non_null!(chain);
        unsafe {
            let chain_ptr = &mut *chain;
            let chain = chain_ptr.take();
            drop(Box::from_raw(chain.unwrap().as_ptr()))
        };
    })
}
