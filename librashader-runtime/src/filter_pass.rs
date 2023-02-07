use librashader_common::ImageFormat;
use librashader_presets::ShaderPassConfig;

pub trait FilterPassMeta {
    fn source_format(&self) -> ImageFormat;
    fn config(&self) -> &ShaderPassConfig;

    fn get_format(&self) -> ImageFormat {
        let fb_format = self.source_format();
        if let Some(format) = self.config().get_format_override() {
            format
        } else if fb_format == ImageFormat::Unknown {
            ImageFormat::R8G8B8A8Unorm
        } else {
            fb_format
        }
    }
}
