//! Shader preset definition (`.slangp`) parser for librashader.
#![feature(drain_filter)]

mod error;
mod parse;
mod preset;
pub use error::*;
pub use preset::*;
