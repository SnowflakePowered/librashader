# librashader

A preprocessor, compiler, and runtime for RetroArch 'slang' shaders, rewritten in pure Rust.

Heavily WIP.

## License
**There is not yet a functioning implementation of librashader but this section outlines its licensing goals in contrast to
RetroArch.**

While librashader is an independent reimplementation of the RetroArch shader pipeline, referencing the RetroArch source
code was indispensable to its creation. As it is therefore considered a derivative work, the core parts of librashader
such as the preprocessor, the preset parser, the reflection library, and the runtimes, are all licensed under GPLv3.

The librashader C API, i.e. its headers and definitions, *not its implementation in `librashader_capi`*,
are unique to librashader and are more permissively licensed, and may allow you to use librashader in your permissively 
licensed or proprietary project.

While the code for `librashader_capi` (`librashader.so` and `rashader.dll`) is still under GPLv3, 
you may use librashader in a non-GPL work by linking against the MIT licensed `librashader_ld`, 
which implements the librashader C API, and thunks its calls to any `librashader.so` or `rashader.dll` 
library found in the load path, *provided that `librashader.so` or `rashader.dll` are distributed under the restrictions
of GPLv3*. 

Note that if your project is not compatible with GPLv3, you **can not distribute `librashader.so` or `rashader.dll`**
alongside your project, **only `librashader-ld.so` or `rashader-ld.dll`**, which will do nothing without a librashader
implementation in the load path. The end user must obtain the implementation of librashader themselves.