//! Shader preset definition parsing for librashader.
//!
//! This crate contains facilities and types for parsing `.slangp` shader presets files.
//!
//! Shader presets contain shader and texture parameters, and the order in which to apply a set of
//! shaders in a filter chain. A librashader runtime takes a resulting [`ShaderPreset`](crate::ShaderPreset)
//! as input to create a filter chain.
//!
//! Re-exported as [`librashader::presets`](https://docs.rs/librashader/latest/librashader/presets/index.html).
#![feature(drain_filter)]

mod error;
mod parse;
mod preset;
mod quark;
mod slang;

pub use error::*;
pub use preset::*;

pub(crate) fn remove_if<T>(values: &mut Vec<T>, f: impl FnMut(&T) -> bool) -> Option<T> {
    values.iter().position(f).map(|idx| values.remove(idx))
}
