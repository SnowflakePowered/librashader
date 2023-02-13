//! This crate implements the librashader transparent cache.
//!
//! This crate is exempt from semantic versioning guarantees and is an implementation
//! detail of librashader runtimes.
#![feature(try_blocks)]
#![feature(once_cell)]
pub mod cache;
pub mod compilation;
pub mod error;

mod key;
mod cacheable;

#[cfg(test)]
mod tests {}

pub use cacheable::Cacheable;
pub use key::CacheKey;

#[cfg(all(target_os = "windows", feature = "d3d"))]
mod d3d;
