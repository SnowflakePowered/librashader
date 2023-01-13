//! The C API for [librashader](https://docs.rs/librashader/).
//!
//! The librashader C API is designed to be loaded dynamically via `librashader_ld.h`, but static usage is also
//! possible by linking against `librashader.h` as well as any static libraries used by `librashader`.
//!
//! ## Usage
//! ⚠ Rust consumers should take a look at [librashader](https://docs.rs/librashader/) ⚠
//!
//! The C API is designed to be easy to use and safe. Most objects are only accessible behind an opaque pointer.
//! Every allocated object can be freed with a corresponding `free` function **for that specific object type**.
//!
//! Once an object is freed, the input pointer is always set to null. Attempting to free an object that was not
//! allocated from `librashader` or trying to free an object with a wrong `free` function results in
//! **immediate undefined behaviour**.
//!
//! In general, all functions will accept null pointers for all parameters. However, passing a null pointer
//! into any function that requires a non-null pointer will result in the function returning an error with code `INVALID_PARAMETER`.
//!
//! All types that begin with an underscore, such as `_libra_error` or `_shader_preset` are handles that
//! can not be constructed validly, and should always be used with pointer indirection via the corresponding `_t` types.
//!
//! All functions have safety invariants labeled `## Safety` that must be upheld. Failure to uphold these invariants
//! will result in **immediate undefined behaviour**. Generally speaking, all pointers passed to functions must be
//! **aligned** regardless of whether or not they are null.
//!
//! ## Booleans
//! Some option structs take `bool` values.
//! Any booleans passed to librashader **must have a bit pattern equivalent to either `1` or `0`**. Any other value will cause
//! **immediate undefined behaviour**.
//!
//! ## Errors
//! The librashader C API provides a robust, reflective error system. Every function returns a `libra_error_t`, which is either
//! a null pointer, or a handle to an opaque allocated error object. If the returned error is null, then the function was successful.
//! Otherwise, error information can be accessed via the `libra_error_` set of APIs. If an error indeed occurs, it may be freed by
//! `libra_error_free`.
#![allow(non_camel_case_types)]
#![feature(try_blocks)]
#![feature(vec_into_raw_parts)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod ctypes;
pub mod error;
mod ffi;
pub mod presets;

#[cfg(feature = "reflect")]
pub mod reflect;

pub mod runtime;
