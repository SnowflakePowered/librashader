#![feature(array_chunks)]
//! Helpers and shared logic for librashader runtime implementations.

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
