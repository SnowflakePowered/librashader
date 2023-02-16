#![cfg(target_os = "windows")]
#![feature(const_trait_impl)]
#![feature(let_chains)]
#![feature(type_alias_impl_trait)]
#![feature(int_roundings)]

mod buffer;
mod descriptor_heap;
mod filter_chain;
mod filter_pass;
mod framebuffer;
mod graphics_pipeline;
mod luts;
mod mipmap;
mod parameters;
mod quad_render;
mod samplers;
mod texture;
mod util;

pub mod options;
pub mod error;

pub use filter_chain::FilterChainD3D12;
pub use texture::D3D12InputImage;
pub use texture::D3D12OutputView;
