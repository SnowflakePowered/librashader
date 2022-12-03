#![allow(non_camel_case_types)]
#![feature(try_blocks)]
#![feature(vec_into_raw_parts)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::os::raw::c_char;

pub mod presets;
pub mod runtime;
pub mod error;
pub mod ctypes;
mod ffi;

pub type PK_s = unsafe extern "C" fn(filename: *const c_char);

#[cfg(feature = "headers")] // c.f. the `Cargo.toml` section
pub fn generate_headers() -> ::std::io::Result<()> {
    ::safer_ffi::headers::builder()
        .to_file("librashader.h")?
        .generate()
}