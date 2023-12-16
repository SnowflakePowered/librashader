//! librashader WGPU runtime
//!
//! This crate should not be used directly.
//! See [`librashader::runtime::wgpu`](https://docs.rs/librashader/latest/librashader/runtime/wgpu/index.html) instead.
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(strict_provenance)]

mod draw_quad;
mod error;
mod filter_chain;
mod filter_pass;
mod graphics_pipeline;
mod samplers;
mod texture;
mod util;
mod framebuffer;
mod luts;

pub use filter_chain::FilterChainWGPU;
pub use filter_pass::FilterPass;
