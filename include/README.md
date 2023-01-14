# librashader C headers

The librashader C headers are unlike the implementations, explicitly licensed under the MIT license.

They are provided for easy integration of librashader in a multi-target C or C++ project that may not have
the necessary hardware or APIs available required for all supported runtimes. 

`librashader.h` can be depended upon to link with `librashader.dll` or `librashader.so` if you wish to link 
with librashader. 

An easier alternative is to use the `librashader_ld.h` header library to load function pointers
from any `librashader.dll` or `librashader.so` implementation in the search path. You should customize this
header file to remove support for any runtimes you do not need.