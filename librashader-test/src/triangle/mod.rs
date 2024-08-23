mod d3d11;

use std::path::Path;

/// Test harness to set up a device, render a triangle, and apply a shader
pub trait TriangleTest {
    /// Render a triangle to an image buffer, applying the provided shader.
    ///
    /// The test should render in linear colour space for proper comparison against
    /// backends.
    fn triangle(
        &self,
        image: impl AsRef<Path>,
        path: impl AsRef<Path>,
        frame_count: usize,
    ) -> anyhow::Result<image::RgbaImage>;
}

#[cfg(test)]
mod test {
    use crate::triangle::TriangleTest;
    use image::codecs::png::PngEncoder;
    use std::fs::File;

    const IMAGE_PATH: &str = "../triangle.png";
    const FILTER_PATH: &str = "../test/shaders_slang/crt/crt-royale.slangp";

    #[test]
    pub fn test_d3d11() -> anyhow::Result<()> {
        let d3d11 = super::d3d11::Direct3D11::new()?;
        let image = d3d11.triangle(IMAGE_PATH, FILTER_PATH, 100)?;

        let out = File::create("out.png")?;
        image.write_with_encoder(PngEncoder::new(out))?;
        Ok(())
    }
}
