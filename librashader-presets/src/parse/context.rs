use once_cell::sync::Lazy;
use regex::bytes::Regex;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::ops::Add;
use std::path::{Component, Path, PathBuf};

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum VideoDriver {
    None = 0,
    GlCore,
    Gl,
    Vulkan,
    Direct3D11,
    Direct3D9Hlsl,
    Direct3D12,
    Metal,
}

impl Display for VideoDriver {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoDriver::None => f.write_str("null"),
            VideoDriver::GlCore => f.write_str("glcore"),
            VideoDriver::Gl => f.write_str("gl"),
            VideoDriver::Vulkan => f.write_str("vulkan"),
            VideoDriver::Direct3D11 => f.write_str("d3d11"),
            VideoDriver::Direct3D9Hlsl => f.write_str("d3d9_hlsl"),
            VideoDriver::Direct3D12 => f.write_str("d3d12"),
            VideoDriver::Metal => f.write_str("metal"),
        }
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum ShaderExtension {
    Slang = 0,
    Glsl,
    Cg,
}

impl Display for ShaderExtension {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderExtension::Slang => f.write_str("slang"),
            ShaderExtension::Glsl => f.write_str("glsl"),
            ShaderExtension::Cg => f.write_str("cg"),
        }
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum PresetExtension {
    Slangp = 0,
    Glslp,
    Cgp,
}

impl Display for PresetExtension {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PresetExtension::Slangp => f.write_str("slangp"),
            PresetExtension::Glslp => f.write_str("glslp"),
            PresetExtension::Cgp => f.write_str("cgp"),
        }
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum Rotation {
    /// Zero
    Zero = 0,
    /// 90 degrees
    Right = 1,
    /// 180 degrees
    Straight = 2,
    /// 270 degrees
    Reflex = 3,
}

impl Add for Rotation {
    type Output = Rotation;

    fn add(self, rhs: Self) -> Self::Output {
        let lhs = self as u32;
        let out = lhs + rhs as u32;
        let out = out % 4;
        match out {
            0 => Rotation::Zero,
            1 => Rotation::Right,
            2 => Rotation::Straight,
            3 => Rotation::Reflex,
            _ => unreachable!(),
        }
    }
}

impl Display for Rotation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Rotation::Zero => f.write_str("0"),
            Rotation::Right => f.write_str("90"),
            Rotation::Straight => f.write_str("180"),
            Rotation::Reflex => f.write_str("270"),
        }
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum Orientation {
    Vertical = 0,
    Horizontal,
}

impl Display for Orientation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Orientation::Vertical => f.write_str("VERT"),
            Orientation::Horizontal => f.write_str("HORZ"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ContextItem {
    ContentDirectory(String),
    CoreName(String),
    GameName(String),
    Preset(String),
    PresetDirectory(String),
    VideoDriver(VideoDriver),
    VideoDriverShaderExtension(ShaderExtension),
    VideoDriverPresetExtension(PresetExtension),
    CoreRequestedRotation(Rotation),
    AllowCoreRotation(bool),
    UserRotation(Rotation),
    FinalRotation(Rotation),
    ScreenOrientation(Rotation),
    ViewAspectOrientation(Orientation),
    CoreAspectOrientation(Orientation),
    ExternContext(String, String),
}

impl ContextItem {
    fn toggle_str(v: bool) -> &'static str {
        if v {
            "ON"
        } else {
            "OFF"
        }
    }

    pub fn key(&self) -> &str {
        match self {
            ContextItem::ContentDirectory(_) => "CONTENT-DIR",
            ContextItem::CoreName(_) => "CORE",
            ContextItem::GameName(_) => "GAME",
            ContextItem::Preset(_) => "PRESET",
            ContextItem::PresetDirectory(_) => "PRESET_DIR",
            ContextItem::VideoDriver(_) => "VID-DRV",
            ContextItem::CoreRequestedRotation(_) => "CORE-REQ-ROT",
            ContextItem::AllowCoreRotation(_) => "VID-ALLOW-CORE-ROT",
            ContextItem::UserRotation(_) => "VID-USER-ROT",
            ContextItem::FinalRotation(_) => "VID-FINAL-ROT",
            ContextItem::ScreenOrientation(_) => "SCREEN-ORIENT",
            ContextItem::ViewAspectOrientation(_) => "VIEW-ASPECT-ORIENT",
            ContextItem::CoreAspectOrientation(_) => "CORE-ASPECT-ORIENT",
            ContextItem::VideoDriverShaderExtension(_) => "VID-DRV-SHADER-EXT",
            ContextItem::VideoDriverPresetExtension(_) => "VID-DRV-PRESET-EXT",
            ContextItem::ExternContext(key, _) => key,
        }
    }
}

impl Display for ContextItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextItem::ContentDirectory(v) => f.write_str(v),
            ContextItem::CoreName(v) => f.write_str(v),
            ContextItem::GameName(v) => f.write_str(v),
            ContextItem::Preset(v) => f.write_str(v),
            ContextItem::PresetDirectory(v) => f.write_str(v),
            ContextItem::VideoDriver(v) => f.write_fmt(format_args!("{}", v)),
            ContextItem::CoreRequestedRotation(v) => {
                f.write_fmt(format_args!("{}-{}", self.key(), v))
            }
            ContextItem::AllowCoreRotation(v) => f.write_fmt(format_args!(
                "{}-{}",
                self.key(),
                ContextItem::toggle_str(*v)
            )),
            ContextItem::UserRotation(v) => f.write_fmt(format_args!("{}-{}", self.key(), v)),
            ContextItem::FinalRotation(v) => f.write_fmt(format_args!("{}-{}", self.key(), v)),
            ContextItem::ScreenOrientation(v) => f.write_fmt(format_args!("{}-{}", self.key(), v)),
            ContextItem::ViewAspectOrientation(v) => {
                f.write_fmt(format_args!("{}-{}", self.key(), v))
            }
            ContextItem::CoreAspectOrientation(v) => {
                f.write_fmt(format_args!("{}-{}", self.key(), v))
            }
            ContextItem::VideoDriverShaderExtension(v) => f.write_fmt(format_args!("{}", v)),
            ContextItem::VideoDriverPresetExtension(v) => f.write_fmt(format_args!("{}", v)),
            ContextItem::ExternContext(_, v) => f.write_fmt(format_args!("{}", v)),
        }
    }
}

/// A builder for preset wildcard context.
///
/// Any items added after will have higher priority
/// when passed to the shader preset parser.
///
/// When passed to the preset parser, the preset parser
/// will automatically add inferred items at lowest priority.
///
/// Any items added by the user will override the automatically
/// inferred items.
#[derive(Debug, Clone)]
pub struct WildcardContext(VecDeque<ContextItem>);

impl WildcardContext {
    /// Create a new wildcard context.
    pub fn new() -> Self {
        Self(VecDeque::new())
    }

    /// Prepend an item to the context builder.
    pub fn prepend_item(&mut self, item: ContextItem) {
        self.0.push_front(item);
    }

    /// Append an item to the context builder.
    /// The new item will take precedence over all items added before it.
    pub fn append_item(&mut self, item: ContextItem) {
        self.0.push_back(item);
    }

    /// Prepend sensible defaults for the given video driver.
    ///
    /// Any values added, either previously or afterwards will not be overridden.
    pub fn add_video_driver_defaults(&mut self, video_driver: VideoDriver) {
        self.0.push_front(ContextItem::VideoDriverPresetExtension(
            PresetExtension::Slangp,
        ));
        self.0.push_front(ContextItem::VideoDriverShaderExtension(
            ShaderExtension::Slang,
        ));
        self.0.push_front(ContextItem::VideoDriver(video_driver));
    }

    /// Prepend default entries from the path of the preset.
    ///
    /// Any values added, either previously or afterwards will not be overridden.
    pub fn add_path_defaults(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        if let Some(preset_name) = path.file_stem() {
            let preset_name = preset_name.to_string_lossy();
            self.0.push_front(ContextItem::Preset(preset_name.into()))
        }

        if let Some(preset_dir_name) = path.parent().and_then(|p| {
            if !p.is_dir() {
                return None;
            };
            p.file_name()
        }) {
            let preset_dir_name = preset_dir_name.to_string_lossy();
            self.0
                .push_front(ContextItem::PresetDirectory(preset_dir_name.into()))
        }
    }

    pub(crate) fn to_hashmap(mut self) -> FxHashMap<String, String> {
        let mut map = FxHashMap::default();
        let last_user_rot = self
            .0
            .iter()
            .rfind(|i| matches!(i, ContextItem::UserRotation(_)));
        let last_core_rot = self
            .0
            .iter()
            .rfind(|i| matches!(i, ContextItem::CoreRequestedRotation(_)));

        let final_rot = match (last_core_rot, last_user_rot) {
            (Some(ContextItem::UserRotation(u)), None) => Some(ContextItem::FinalRotation(*u)),
            (None, Some(ContextItem::CoreRequestedRotation(c))) => {
                Some(ContextItem::FinalRotation(*c))
            }
            (Some(ContextItem::UserRotation(u)), Some(ContextItem::CoreRequestedRotation(c))) => {
                Some(ContextItem::FinalRotation(*u + *c))
            }
            _ => None,
        };

        if let Some(final_rot) = final_rot {
            self.prepend_item(final_rot);
        }

        for item in self.0 {
            map.insert(String::from(item.key()), item.to_string());
        }

        map
    }
}

pub fn apply_context(path: &mut PathBuf, context: &FxHashMap<String, String>) {
    static WILDCARD_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("\\$([A-Z-_]+)\\$").unwrap());
    if context.is_empty() {
        return;
    }
    // Don't want to do any extra work if there's no match.
    if !WILDCARD_REGEX.is_match(path.as_os_str().as_encoded_bytes()) {
        return;
    }

    let mut new_path = PathBuf::with_capacity(path.capacity());
    for component in path.components() {
        match component {
            Component::Normal(path) => {
                let haystack = path.as_encoded_bytes();
                let replaced =
                    WILDCARD_REGEX.replace_all(haystack, |caps: &regex::bytes::Captures| {
                        let Some(name) = caps.get(1) else {
                            return caps[0].to_vec();
                        };

                        let Ok(key) = std::str::from_utf8(name.as_bytes()) else {
                            return caps[0].to_vec();
                        };
                        if let Some(replacement) = context.get(key) {
                            return OsString::from(replacement.to_string()).into_encoded_bytes();
                        }
                        return caps[0].to_vec();
                    });

                // SAFETY: The original source is valid encoded bytes, and our replacement is
                // valid encoded bytes. This upholds the safety requirements of `from_encoded_bytes_unchecked`.
                new_path.push(unsafe { OsStr::from_encoded_bytes_unchecked(&replaced.as_ref()) })
            }
            _ => new_path.push(component),
        }
    }

    // If no wildcards are found within the path, or the path after replacing the wildcards does not exist on disk, the path returned will be unaffected.
    if let Ok(true) = new_path.try_exists() {
        *path = new_path;
    }
}
