pub mod d3d11;
pub mod gl;
pub mod vk;
pub mod wgpu;

use std::path::Path;

/// Test harness to set up a device, render a triangle, and apply a shader
pub trait RenderTest {
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

    #[test]
    pub fn test_d3d11() -> anyhow::Result<()> {
        let d3d11 = super::d3d11::Direct3D11::new(IMAGE_PATH)?;
        let image = d3d11.render(FILTER_PATH, 100)?;

        let out = File::create("out.png")?;
        image.write_with_encoder(PngEncoder::new(out))?;
        Ok(())
    }

    #[test]
    pub fn test_wgpu() -> anyhow::Result<()> {
        let wgpu = super::wgpu::Wgpu::new(IMAGE_PATH)?;
        let image = wgpu.render(FILTER_PATH, 100)?;

        let out = File::create("out.png")?;
        image.write_with_encoder(PngEncoder::new(out))?;
        Ok(())
    }

    #[test]
    pub fn test_vk() -> anyhow::Result<()> {
        let vulkan = super::vk::Vulkan::new(IMAGE_PATH)?;
        let image = vulkan.render(FILTER_PATH, 100)?;
        //
        let out = File::create("out.png")?;
        image.write_with_encoder(PngEncoder::new(out))?;
        Ok(())
    }

    #[test]
    pub fn test_gl3() -> anyhow::Result<()> {

        let gl = super::gl::OpenGl3::new(IMAGE_PATH)?;
        let image = gl.render(FILTER_PATH, 1000)?;

        let out = File::create("out.png")?;
        image.write_with_encoder(PngEncoder::new(out))?;
        Ok(())
    }

    #[test]
    pub fn test_gl4() -> anyhow::Result<()> {

        let gl = super::gl::OpenGl4::new(IMAGE_PATH)?;
        let image = gl.render(FILTER_PATH, 1000)?;

        let out = File::create("out.png")?;
        image.write_with_encoder(PngEncoder::new(out))?;
        Ok(())
    }

    #[test]
    pub fn compare() -> anyhow::Result<()> {
        let d3d11 = super::d3d11::Direct3D11::new(IMAGE_PATH)?;
        let wgpu = super::wgpu::Wgpu::new(IMAGE_PATH)?;

        let wgpu_image = wgpu.render(FILTER_PATH, 100)?;
        let d3d11_image = d3d11.render(FILTER_PATH, 100)?;

        let similarity = image_compare::rgba_hybrid_compare(&wgpu_image, &d3d11_image)?;

        assert!(similarity.score > 0.95);

        Ok(())
    }
}
