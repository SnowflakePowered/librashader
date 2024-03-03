#![cfg(target_os = "windows")]
#![feature(type_alias_impl_trait)]
#![feature(error_generic_member_access)]
mod binding;
mod draw_quad;
mod filter_chain;
mod filter_pass;
mod graphics_pipeline;
mod luts;
mod samplers;
mod texture;
mod util;
mod d3dx;
pub mod error;
pub mod options;

use librashader_runtime::impl_filter_chain_parameters;
impl_filter_chain_parameters!(FilterChainD3D9);

pub use crate::filter_chain::FilterChainD3D9;
