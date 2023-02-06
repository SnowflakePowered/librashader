# librashader

![Mega Bezel SMOOTH-ADV](https://user-images.githubusercontent.com/1000503/212806508-11e6942d-ac48-4659-bd61-1e50259d92b2.png)

<small>*Mega Bezel SMOOTH-ADV on DirectX 11*</small>

librashader (*/ˈli:brəʃeɪdɚ/*) is a preprocessor, compiler, and runtime for RetroArch 'slang' shaders, rewritten in pure Rust.

[![Latest Version](https://img.shields.io/crates/v/librashader.svg)](https://crates.io/crates/librashader) [![Docs](https://docs.rs/librashader/badge.svg)](https://docs.rs/librashader) ![License](https://img.shields.io/crates/l/librashader)
![Nightly rust](https://img.shields.io/badge/rust-nightly-orange.svg)

## Supported Render APIs
librashader supports OpenGL 3, OpenGL 4.6, Vulkan, Direct3D 11, and Direct3D 12.  Older versions
of Direct3D and OpenGL, as well as Metal, are not supported (but pull-requests are welcome).

| **API**     | **Status** | **`librashader` feature** |
|-------------|------------|---------------------------|
| OpenGL 3.3+ | ✔          | `gl`                      |
| OpenGL 4.6  | ✔          | `gl`                      |
| Vulkan      | ✔          | `vk`                      |
| Direct3D 11 | ✔          | `d3d11`                   |
| Direct3D 12 | ✔          | `d3d12`                   |
| OpenGL 2    | ❌          |                           |
| Direct3D 9  | ❌          |                           |
| Direct3D 10 | ❌          |                           |
| Metal       | ❌          |                           |


✔ = Render API is supported &mdash; 🚧 =  Support is in progress &mdash; ❌ Render API is not supported
## Usage

librashader provides both a Rust API under the `librashader` crate, and a C API. Both APIs are first-class and fully supported.
The C API is geared more towards integration with existing projects. The Rust `librashader` crate exposes more
of the internals if you wish to use parts of librashader piecemeal.

The librashader C API is best used by linking statically with `librashader_ld`, which implements a loader that dynamically
loads the librashader (`librashader.so` or `librashader.dll`) implementation in the search path. 

### Building

For Rust projects, simply add the crate to your `Cargo.toml`
```
cargo add librashader
```

To build the C compatible dynamic library, [cargo-post](https://crates.io/crates/cargo-post) is required.

```
cargo post build --release --package librashader-capi
```

This will output a `librashader.dll` or `librashader.so` in the target folder.

### C ABI Compatibility
The recommended way of integrating `librashader` is by the `librashader_ld` single header library, ABI stability 
is important to ensure that updates to librashader do not break existing consumers.

Pre-1.0, nothing is guaranteed to be stable, but the following APIs are unlikely to change their ABI unless otherwise indicated.

* `libra_preset_*`
* `libra_error_*`

The following APIs, mostly runtime, are more likely to change their ABI before a 1.0 release as I experiment with what
works best.

* `libra_gl_*`
* `libra_vk_*`
* `libra_d3d11_*`
* `libra_d3d12_*`

Linking against `librashader.h` directly is possible, but is not officially supported. You will need to ensure linkage
parameters are correct in order to successfully link with `librashader.lib` or `librashader.a`. The [corrosion](https://github.com/corrosion-rs/)
CMake package is highly recommended.

### Examples

The following Rust examples show how to use each librashader runtime.
* [Vulkan](https://github.com/SnowflakePowered/librashader/blob/master/librashader-runtime-vk/src/lib.rs)
* [OpenGL](https://github.com/SnowflakePowered/librashader/blob/master/librashader-runtime-gl/src/lib.rs)
* [Direct3D 11](https://github.com/SnowflakePowered/librashader/blob/master/librashader-runtime-d3d11/src/lib.rs)
* [Direct3D 12](https://github.com/SnowflakePowered/librashader/blob/master/librashader-runtime-d3d12/src/lib.rs)

Some basic examples on using the C API are also provided in the [librashader-capi-tests](https://github.com/SnowflakePowered/librashader/tree/master/test/capi-tests/librashader-capi-tests)
directory.

## Compatibility

librashader implements the entire RetroArch shader pipeline and is highly compatible with existing shaders.

Please report an issue if you run into a shader that works in RetroArch, but not under librashader.

* Filter chains do not terminate at the backbuffer.
  * Unlike RetroArch, librashader does not have full knowledge of the entire rendering state and is designed to be pluggable
    at any point in your render pipeline. Instead, filter chains terminate at a caller-provided output surface and viewport. 
    It is the caller's responsibility to blit the surface back to the backbuffer.
* Shaders are compiled in parallel where possible. This should noticeably decrease preset compile times. Note that parallel compilation
  is not available to OpenGL.
* Runtime-specific differences
  * OpenGL
    * Copying of in-flight framebuffer contents to history is done via `glBlitFramebuffer` rather than drawing a quad into an intermediate FBO.
    * Sampler objects are used rather than `glTexParameter`.
    * Sampler inputs and outputs are not renamed. This is useful for debugging shaders in RenderDoc.
    * UBO and Push Constant Buffer sizes are padded to 16-byte boundaries.
  * OpenGL 4.6+
    * All caveats from the OpenGL 3.3+ section should be considered.
    * Should work on OpenGL 4.5 but this is not guaranteed. The OpenGL 4.6 runtime may eventually switch to using `ARB_spirv_extensions` for loading shaders, and this will not be marked as a breaking change.
    * The OpenGL 4.6 runtime uses Direct State Access to minimize changes to the OpenGL state. For GPUs released within the last 5 years, this may improve performance.
  * Vulkan 1.3+
    * The Vulkan runtime uses [`VK_KHR_dynamic_rendering`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_dynamic_rendering.html) by default.
      This extension must be enabled at device creation. Explicit render passes can be used by configuring filter chain options, but may have reduced performance 
      compared to dynamic rendering.
    * UBOs use multiple discontiguous buffers. This may be improved in the future by switching to VMA rather than manually handling allocations.
  * Direct3D 11
    * Framebuffer copies are done via `ID3D11DeviceContext::CopySubresourceRegion` rather than a CPU conversion + copy.
    * HDR10 support is not part of the shader runtime and is not supported.
  * Direct3D 12
    * The Direct3D 12 runtime uses [render passes](https://learn.microsoft.com/en-us/windows/win32/direct3d12/direct3d-12-render-passes). This feature has been available since Windows 10 version 1809,
      which was released in late 2018.
    * For maximum compatibility with shaders, a shader compile pipeline based on [`spirv-to-dxil`](https://github.com/SnowflakePowered/spirv-to-dxil-rs) is used, with the SPIRV-Cross HLSL pipeline used as a fallback. 
      This brings shader compatibility beyond what the RetroArch Direct3D 12 runtime provides. The HLSL pipeline fallback may be removed in the future as`spirv-to-dxil` improves.
    
Most, if not all shader presets should work fine on librashader. The runtime specific differences should not affect the output,
and are more a heads-up for integrating librashader into your project.

### Writing a librashader Runtime

If you wish to contribute a runtime implementation not already available, see the [librashader-runtime](https://docs.rs/librashader-runtime/latest/librashader_runtime/)
crate for helpers and shared logic used across all librashader runtime implementations. Using these helpers and traits will
ensure that your runtime has consistent behaviour for uniform and texture semantics bindings with the existing librashader runtimes.

These types should not be exposed to the end user in the runtime's public API, and should be kept internal to the implementation of 
the runtime.

## License
The core parts of librashader such as the preprocessor, the preset parser, 
the reflection library, and the runtimes, are all licensed under the Mozilla Public License version 2.0.

The librashader C API, i.e. its headers and definitions, *not its implementation in `librashader-capi`*,
are more permissively licensed, and may allow you to use librashader in your permissively 
licensed or proprietary project.

To facilitate easier use of librashader in projects incompatible with MPL-2.0, `librashader_ld`
implements a loader which thunks its calls to any `librashader.so` or `librashader.dll`
library found in the load path. A non-MPL-2.0 compatible project may link against
`librashader_ld` to use the librashader runtime, *provided that `librashader.so` or `librashader.dll` 
are distributed under the restrictions of MPLv2*.

Note that this means that if your project is unable to comply with the requirements of MPL-2.0, 
you **can not distribute `librashader.so` or `librashader.dll`** alongside your project. 
The end user must obtain the implementation of librashader themselves. For more information, 
see the [MPL 2.0 FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).


At your discretion, you may instead choose to distribute `librashader` under the terms of GPLv3 rather than MPL-2.0.