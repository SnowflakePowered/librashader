#![cfg(target_os = "windows")]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(let_chains)]
#![feature(type_alias_impl_trait)]

mod buffer;
mod descriptor_heap;
mod filter_chain;
mod filter_pass;
mod framebuffer;
mod graphics_pipeline;
mod luts;
mod mipmap;
mod parameters;
mod draw_quad;
mod samplers;
mod texture;
mod util;

pub mod error;
pub mod options;

pub use filter_chain::FilterChainD3D12;
pub use texture::D3D12InputImage;
pub use texture::D3D12OutputView;
