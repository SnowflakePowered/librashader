//! librashader runtime C APIs
#[cfg(feature = "runtime-opengl")]
pub mod gl;

#[cfg(any(
    feature = "docsrs",
    all(target_os = "windows", feature = "runtime-d3d11")
))]
pub mod d3d11;

#[cfg(feature = "runtime-vulkan")]
pub mod vk;
