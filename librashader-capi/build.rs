pub fn main() {
    #[cfg(all(target_os = "windows", feature = "runtime-d3d12"))]
    {
        println!("cargo:rustc-link-lib=dylib=delayimp");
        println!("cargo:rustc-link-arg=/DELAYLOAD:d3d12.dll");
    }
    #[cfg(all(
        target_os = "windows",
        feature = "runtime-d3d12",
        not(all(feature = "runtime-d3d12-static", target_arch = "x86_64"))
    ))]
    {
        println!("cargo:rustc-link-arg=/DELAYLOAD:dxcompiler.dll");
    }
}
