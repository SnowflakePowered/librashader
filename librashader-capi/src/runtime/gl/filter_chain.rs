use crate::ctypes::{libra_gl_filter_chain_t, libra_shader_preset_t, libra_viewport_t};
use crate::error::{assert_non_null, assert_some_ptr, LibrashaderError};
use crate::ffi::extern_fn;
use librashader::runtime::gl::{Framebuffer, GLImage};
use std::ffi::CStr;
use std::ffi::{c_char, c_void, CString};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::slice;

pub use librashader::runtime::gl::capi::options::FilterChainOptionsGL;
pub use librashader::runtime::gl::capi::options::FrameOptionsGL;
use librashader::runtime::FilterChainParameters;
use librashader::runtime::{Size, Viewport};

/// A GL function loader that librashader needs to be initialized with.
pub type libra_gl_loader_t = unsafe extern "system" fn(*const c_char) -> *const c_void;

/// OpenGL parameters for the source image.
#[repr(C)]
pub struct libra_source_image_gl_t {
    /// A texture GLuint to the source image.
    pub handle: u32,
    /// The format of the source image.
    pub format: u32,
    /// The height of the source image.
    pub height: u32,
    /// The width of the source image.
    pub width: u32,
}

/// OpenGL parameters for the output framebuffer.
#[repr(C)]
pub struct libra_draw_framebuffer_gl_t {
    /// A framebuffer GLuint to the output framebuffer.
    pub handle: u32,
    /// A texture GLuint to the logical buffer of the output framebuffer.
    pub texture: u32,
    /// The format of the output framebuffer.
    pub format: u32,
}

impl From<libra_source_image_gl_t> for GLImage {
    fn from(value: libra_source_image_gl_t) -> Self {
        GLImage {
            handle: value.handle,
            format: value.format,
            size: Size::new(value.width, value.height),
            padded_size: Size::default(),
        }
    }
}

extern_fn! {
    /// Initialize the OpenGL Context for librashader.
    ///
    /// This only has to be done once throughout the lifetime of the application,
    /// unless for whatever reason you switch OpenGL loaders mid-flight.
    ///
    /// ## Safety
    /// Attempting to create a filter chain will fail if the GL context is not initialized.
    ///
    /// Reinitializing the OpenGL context with a different loader immediately invalidates previous filter
    /// chain objects, and drawing with them causes immediate undefined behaviour.
    raw fn libra_gl_init_context(loader: libra_gl_loader_t) {
        gl::load_with(|s| unsafe {
            let proc_name = CString::new(s).unwrap_unchecked();
            loader(proc_name.as_ptr())
        });

        LibrashaderError::ok()
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
    fn libra_gl_filter_chain_create(
        preset: *mut libra_shader_preset_t,
        options: *const FilterChainOptionsGL,
        out: *mut MaybeUninit<libra_gl_filter_chain_t>
    ) {
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

        let chain = librashader::runtime::gl::capi::FilterChainGL::load_from_preset(*preset, options)?;

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
    fn libra_gl_filter_chain_frame(
        chain: *mut libra_gl_filter_chain_t,
        frame_count: usize,
        image: libra_source_image_gl_t,
        viewport: libra_viewport_t,
        out: libra_draw_framebuffer_gl_t,
        mvp: *const f32,
        opt: *const FrameOptionsGL
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        let image: GLImage = image.into();
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
        let framebuffer = Framebuffer::new_from_raw(out.texture, out.handle, out.format, Size::new(viewport.width, viewport.height), 1);
        let viewport = Viewport {
            x: viewport.x,
            y: viewport.y,
            output: &framebuffer,
            mvp,
        };
        chain.frame(&image, &viewport, frame_count, opt.as_ref())?;
    }
}

extern_fn! {
    /// Sets a parameter for the filter chain.
    ///
    /// If the parameter does not exist, returns an error.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_gl_filter_chain_t`.
    /// - `param_name` must be either null or a null terminated string.
    fn libra_gl_filter_chain_set_param(
        chain: *mut libra_gl_filter_chain_t,
        param_name: *const c_char,
        value: f32
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        assert_non_null!(param_name);
        unsafe {
            let name = CStr::from_ptr(param_name);
            let name = name.to_str()?;

            if chain.set_parameter(name, value).is_none() {
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
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_gl_filter_chain_t`.
    /// - `param_name` must be either null or a null terminated string.
    fn libra_gl_filter_chain_get_param(
        chain: *mut libra_gl_filter_chain_t,
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
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_gl_filter_chain_t`.
    fn libra_gl_filter_chain_set_active_pass_count(
        chain: *mut libra_gl_filter_chain_t,
        value: u32
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        chain.set_enabled_pass_count(value as usize);
    }
}

extern_fn! {
    /// Gets the number of active passes for this chain.
    ///
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_gl_filter_chain_t`.
    fn libra_gl_filter_chain_get_active_pass_count(
        chain: *mut libra_gl_filter_chain_t,
        out: *mut MaybeUninit<u32>
    ) mut |chain| {
        assert_some_ptr!(mut chain);
        let value = chain.get_enabled_pass_count();
        unsafe {
            out.write(MaybeUninit::new(value as u32))
        }
    }
}

extern_fn! {
    /// Free a GL filter chain.
    ///
    /// The resulting value in `chain` then becomes null.
    /// ## Safety
    /// - `chain` must be either null or a valid and aligned pointer to an initialized `libra_gl_filter_chain_t`.
    fn libra_gl_filter_chain_free(
        chain: *mut libra_gl_filter_chain_t
    ) {
        assert_non_null!(chain);
        unsafe {
            let chain_ptr = &mut *chain;
            let chain = chain_ptr.take();
            drop(Box::from_raw(chain.unwrap().as_ptr()))
        };
    }
}
