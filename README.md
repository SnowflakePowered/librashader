# librashader

![image](https://user-images.githubusercontent.com/1000503/202991618-e3e38e05-f0de-429d-a3ee-4cd0b077f88f.png)

<small>*crt-royale-fake-bloom*</small>

A preprocessor, compiler, and runtime for RetroArch 'slang' shaders, rewritten in pure Rust.

Heavily WIP.

## Supported Render APIs
librashader supports OpenGL 3, Vulkan, DirectX 11, and DirectX 12. Support is WIP for all runtimes except OpenGL 3. Older versions
of DirectX and OpenGL, as well as Metal, are not supported (but pull-requests are welcome).

| **API**    | **Status** | **`librashader` feature** |
|------------|------------|--------------------------|
| OpenGL 3+  | ✔          | `gl`                     |
| Vulkan     | 🚧         | `vk`                     |
| Direct3D11 | 🚧         | `d3d11`                  |
| Direct3D12 | 🚧         | `d3d12`                  |
| OpenGL 2   | ❌          |                          |
| DirectX 9  | ❌          |                          |
| Metal      | ❌          |                          |

## Usage

🚧 *`librashader_ld` is WIP* 🚧

librashader provides both a Rust API under the `librashader` crate, and a C API. Both APIs are first-class, fully supported.

The librashader C API is best used by linking statically with `librashader_ld`, which implements a loader that dynamically
loads the librashader (`librashader.so` or `rashader.dll`) implementation in the search path.

## License
The core parts of librashader such as the preprocessor, the preset parser, 
the reflection library, and the runtimes, are all licensed under the Mozilla Public License version 2.0.

The librashader C API, i.e. its headers and definitions, *not its implementation in `librashader_capi`*,
are more permissively licensed, and may allow you to use librashader in your permissively 
licensed or proprietary project.

To facilitate easier use of librashader in projects incompatible with MPL-2.0, `librashader_ld`
implements a loader which thunks its calls to any `librashader.so` or `rashader.dll`
library found in the load path. A non-MPL-2.0 compatible project may link against
`librashader_ld` to use the librashader runtime, *provided that `librashader.so` or `rashader.dll` 
are distributed under the restrictions of MPLv2*.

Note that this means that if your project is not compatible with MPL-2.0, you **can not distribute `librashader.so` or `rashader.dll`**
alongside your project. The end user must obtain the implementation of librashader themselves.

At your discretion, you may instead choose to distribute `librashader` under the terms of GPLv3 rather than MPL-2.0