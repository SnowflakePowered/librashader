# librashader

![gameboy-player-crt-royale](https://user-images.githubusercontent.com/1000503/211993121-2ec1f6f0-445b-4b47-8612-291a4eab5d15.png)

<small>*Mega Bezel SMOOTH-ADV on OpenGL 4.6*</small>

librashader (*/ˈli:brəʃeɪdɚ/*) is a preprocessor, compiler, and runtime for RetroArch 'slang' shaders, rewritten in pure Rust.

Heavily WIP.

## Supported Render APIs
librashader supports OpenGL 3, OpenGL 4.6, Vulkan, DirectX 11, and DirectX 12.  Older versions
of DirectX and OpenGL, as well as Metal, are not supported (but pull-requests are welcome).

| **API**     | **Status** | **`librashader` feature** |
|-------------|------------|---------------------------|
| OpenGL 3.3+ | ✔          | `gl`                      |
| OpenGL 4.6  | ✔          | `gl`                      |
| Vulkan 1.3+ | ✔         | `vk`                      |
| Direct3D11  | ✔          | `d3d11`                   |
| Direct3D12  | 🚧         | `d3d12`                   |
| OpenGL 2    | ❌          |                           |
| DirectX 9   | ❌          |                           |
| Metal       | ❌          |                           |

✔ = Render API is supported &mdash; 🚧 =  Support is in progress &mdash; ❌ Render API is not supported
## Usage

🚧 *`librashader_ld` is WIP* 🚧

librashader provides both a Rust API under the `librashader` crate, and a C API. Both APIs are first-class, fully supported.

The librashader C API is best used by linking statically with `librashader_ld`, which implements a loader that dynamically
loads the librashader (`librashader.so` or `librashader.dll`) implementation in the search path.

Note that the Rust crate requires nightly Rust to build.

### C ABI Compatibility
Since the recommended way of integrating `librashader` is by the `librashader_ld` single header library, ABI stability 
is important to ensure that updates to librashader do not break existing consumers.

Pre-1.0, nothing is guaranteed to be stable, but the following APIs are unlikely to change their ABI.

* `libra_preset_*`
* `libra_error_*`

The following APIs, mostly runtime, are more likely to change their ABI.

* `libra_gl_*`
* `libra_vk_*`
* `libra_d3d11_*`
* `libra_d3d12_*`

## Compatibility

librashader implements the entire RetroArch shader pipeline and is highly compatible with existing shaders,
but there are some deliberate differences in design choices that may potentially cause incompatiblities with certain
shaders.

Please report an issue if you run into a shader that works in RetroArch, but not under librashader.

* Variables can only be bound in *either* the UBO or push constant block.
  * RetroArch allows a variable to be present in both a push constant block and a UBO block at the same time. To make the 
    implementation a little cleaner, librashader only allows a variable to be in *either* a push constant block or a UBO
    block. As far as I have tested, there are no shaders in [libretro/slang-shaders](https://github.com/libretro/slang-shaders)
    that bind the same variable in both the push constant and the UBO block.
* Filter chains do not terminate at the backbuffer.
  * Unlike RetroArch, librashader does not have full knowledge of the entire rendering state and is designed to be pluggable
    at any point in your render pipeline. Instead, filter chains terminate at a caller-provided output surface and viewport. 
    It is the caller's responsibility to blit the surface back to the backbuffer.
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
  * Direct3D 11
    * Framebuffer copies are done via `ID3D11DeviceContext::CopySubresourceRegion` rather than a CPU conversion + copy.
    * HDR10 support is not part of the shader runtime and is not supported.
  * Vulkan 1.3+
    * The Vulkan runtime uses [`VK_KHR_dynamic_rendering`](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_dynamic_rendering.html). This extension must be enabled at device creation to use librashader.
    * UBOs use multiple discontiguous buffers. This may be improved in the future by switching to VMA rather than manually handling allocations.

Most, if not all shader presets should work fine on librashader. The runtime specific differences should not affect the output,
and are more a heads-up for integrating librashader into your project.

Compatibility issues may arise with framebuffer copies for original history, but I have not found any yet; 
if it does end up that this results in actual rendering differences I may change the implementation to be more in line
with RetroArch's copy implementation. However, since the Vulkan runtime already uses `vkCmdCopyImage` it is likely that it will
not cause issues.

## License
The core parts of librashader such as the preprocessor, the preset parser, 
the reflection library, and the runtimes, are all licensed under the Mozilla Public License version 2.0.

The librashader C API, i.e. its headers and definitions, *not its implementation in `librashader_capi`*,
are more permissively licensed, and may allow you to use librashader in your permissively 
licensed or proprietary project.

To facilitate easier use of librashader in projects incompatible with MPL-2.0, `librashader_ld`
implements a loader which thunks its calls to any `librashader.so` or `librashader.dll`
library found in the load path. A non-MPL-2.0 compatible project may link against
`librashader_ld` to use the librashader runtime, *provided that `librashader.so` or `librashader.dll` 
are distributed under the restrictions of MPLv2*.

Note that this means that if your project is not compatible with MPL-2.0, you **can not distribute `librashader.so` or `librashader.dll`**
alongside your project. The end user must obtain the implementation of librashader themselves.

At your discretion, you may instead choose to distribute `librashader` under the terms of GPLv3 rather than MPL-2.0