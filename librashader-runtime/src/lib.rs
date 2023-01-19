//! Helpers and shared logic for librashader runtime implementations.
//!
//! Most of this is code internal to librashader runtime implementations and is not
//! intended for general use unless writing a librashader runtime.
//!
//! This crate is exempt from semantic versioning of the librashader API.
#![feature(array_chunks)]

/// Scaling helpers.
pub mod scaling;

/// Semantics helpers.
pub mod semantics;

/// Uniform binding helpers.
pub mod uniforms;

/// Parameter reflection helpers and traits.
pub mod parameters;

/// Image handling helpers.
pub mod image;

/// Ringbuffer helpers
pub mod ringbuffer;

/// Generic implementation of semantics binding.
pub mod binding;

/// Generic helpers for loading shader passes into compiled shader targets and semantics.
pub mod reflect;
