//! Helpers and shared logic for librashader runtime implementations.
//!
//! Most of this is code internal to librashader runtime implementations and is not
//! intended for general use unless writing a librashader runtime.
//!
//! This crate is exempt from semantic versioning of the librashader API.
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
