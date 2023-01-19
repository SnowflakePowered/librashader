//! Helpers and shared logic for librashader runtime implementations.
//!
//! Most of this is only useful when _writing_ a librashader runtime implementations,
//! not _using_ a librashader runtime. Types useful for _using_ the runtime implementations
//! will be re-exported in [`librashader::runtime`](https://docs.rs/librashader/latest/librashader/runtime/index.html).
//!
//! If you are _writing_ a librashader runtime implementation, using these traits and helpers will
//! help in maintaining consistent behaviour in binding semantics and image handling.
#![feature(array_chunks)]

/// Scaling helpers.
pub mod scaling;

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

/// Used to declare a `ShaderPassMeta` type for the given target shader language and compilation type.
#[macro_export]
macro_rules! decl_shader_pass_meta {
    (type $ty_name:ident = <$target:ty, $compilation:ty>) => {
        type $ty_name =
            librashader_reflect::reflect::presets::ShaderPassMeta<
                impl librashader_reflect::back::CompileShader<
                        $target,
                        Options = <$target as librashader_reflect::back::FromCompilation<
                            $compilation,
                        >>::Options,
                        Context = <$target as librashader_reflect::back::FromCompilation<
                            $compilation,
                        >>::Context,
                    > + librashader_reflect::reflect::ReflectShader,
            >;
    };
}
