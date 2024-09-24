#[cfg(feature = "d3d11")]
pub mod d3d11;

#[cfg(feature = "d3d12")]
pub mod d3d12;

#[cfg(feature = "opengl")]
pub mod gl;

#[cfg(feature = "vulkan")]
pub mod vk;

#[cfg(feature = "wgpu")]
pub mod wgpu;

#[cfg(feature = "metal")]
pub mod mtl;

use std::path::Path;

/// Test harness to set up a device, render a triangle, and apply a shader
pub trait RenderTest {
    /// Create a new instance of the test harness.
    fn new(path: impl AsRef<Path>) -> anyhow::Result<Self>
    where
        Self: Sized;

    /// Render a shader onto an image buffer, applying the provided shader.
    ///
    /// The test should render in linear colour space for proper comparison against
    /// backends.
    ///
    /// For testing purposes, it is often that a single image will be reused with multiple
    /// shader presets, so the actual image that a shader will be applied to
    /// will often be part of the test harness object.
    fn render(
        &self,
        path: impl AsRef<Path>,
        frame_count: usize,
    ) -> anyhow::Result<image::RgbaImage>;
}

#[cfg(test)]
mod test {

    use crate::render::RenderTest;
    use image::codecs::png::PngEncoder;
    use std::fs::File;

    const IMAGE_PATH: &str = "../triangle.png";
    const FILTER_PATH: &str = "../test/shaders_slang/crt/crt-royale.slangp";

    // const FILTER_PATH: &str =
    //     "../test/shaders_slang/bezel/Mega_Bezel/Presets/MBZ__0__SMOOTH-ADV.slangp";

    fn do_test<T: RenderTest>() -> anyhow::Result<()> {
        let test = T::new(IMAGE_PATH)?;
        let image = test.render(FILTER_PATH, 100)?;

        let out = File::create("out.png")?;
        image.write_with_encoder(PngEncoder::new(out))?;
        Ok(())
    }

    #[test]
    #[cfg(feature = "d3d11")]
    pub fn test_d3d11() -> anyhow::Result<()> {
        do_test::<crate::render::d3d11::Direct3D11>()
    }

    #[test]
    #[cfg(feature = "wgpu")]
    pub fn test_wgpu() -> anyhow::Result<()> {
        do_test::<crate::render::wgpu::Wgpu>()
    }

    #[test]
    #[cfg(feature = "vulkan")]
    pub fn test_vk() -> anyhow::Result<()> {
        do_test::<crate::render::vk::Vulkan>()
    }

    #[test]
    #[cfg(feature = "opengl")]
    pub fn test_gl3() -> anyhow::Result<()> {
        do_test::<crate::render::gl::OpenGl3>()
    }

    #[test]
    #[cfg(feature = "opengl")]
    pub fn test_gl4() -> anyhow::Result<()> {
        do_test::<crate::render::gl::OpenGl4>()
    }

    #[test]
    #[cfg(feature = "metal")]
    pub fn test_metal() -> anyhow::Result<()> {
        do_test::<crate::render::mtl::Metal>()
    }

    pub fn compare<A: RenderTest, B: RenderTest>() -> anyhow::Result<()> {
        let a = A::new(IMAGE_PATH)?;
        let b = B::new(IMAGE_PATH)?;

        let a_image = a.render(FILTER_PATH, 100)?;
        let b_image = b.render(FILTER_PATH, 100)?;

        let similarity = image_compare::rgba_hybrid_compare(&a_image, &b_image)?;
        assert!(similarity.score > 0.95);
        Ok(())
    }
}
