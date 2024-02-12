#![cfg(target_vendor = "apple")]
#![feature(type_alias_impl_trait)]

mod buffer;
mod draw_quad;
mod filter_chain;
mod filter_pass;
mod graphics_pipeline;
mod luts;
mod samplers;
mod texture;

pub use filter_chain::FilterChainMetal;

pub mod error;
pub mod options;
use librashader_runtime::impl_filter_chain_parameters;
impl_filter_chain_parameters!(FilterChainMetal);
