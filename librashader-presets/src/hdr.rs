use crate::ShaderPreset;
use librashader_common::{ColorSpace, ImageFormat};
use librashader_preprocess::{PreprocessError, ShaderSource};

/// Query the output color space of a shader preset.
pub trait PresetColorSpace {
    /// Get the target color space of the final pass, derived from its declared
    /// output format.
    ///
    /// Returns:
    /// - [`ColorSpace::Hdr10`] if the final pass writes `A2B10G10R10_UNORM_PACK32`,
    /// - [`ColorSpace::ScRgb`] if the final pass writes `R16G16B16A16_SFLOAT`,
    /// - [`ColorSpace::Sdr`] otherwise (or for empty presets).
    ///
    /// If the host intends a [`ColorSpace::PqScRgb`] swapchain, the preset color space must be
    /// [`ColorSpace::ScRgb`] for successful promotion.
    fn color_space(&self) -> Result<ColorSpace, PreprocessError>;
}

impl PresetColorSpace for ShaderPreset {
    fn color_space(&self) -> Result<ColorSpace, PreprocessError> {
        let Some(last) = self.passes.last() else {
            return Ok(ColorSpace::Sdr);
        };

        let effective_format = if let Some(over) = last.meta.get_format_override() {
            over
        } else {
            ShaderSource::load(last.path.as_path(), self.features)?.format
        };

        Ok(match effective_format {
            ImageFormat::A2B10G10R10UnormPack32 => ColorSpace::Hdr10,
            ImageFormat::R16G16B16A16Sfloat => ColorSpace::ScRgb,
            _ => ColorSpace::Sdr,
        })
    }
}
