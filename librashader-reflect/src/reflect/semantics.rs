use crate::reflect::ReflectMeta;
use bitflags::bitflags;
use rustc_hash::FxHashMap;
use std::str::FromStr;

pub const BASE_SEMANTICS_COUNT: usize = 5;
pub const MAX_BINDINGS_COUNT: u32 = 16;
pub const MAX_PUSH_BUFFER_SIZE: u32 = 128;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Hash)]
pub enum UniformType {
    MVP,
    Size,
    Unsigned,
    Signed,
    Float,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Hash)]
#[repr(i32)]
pub enum VariableSemantics {
    // mat4, MVP
    MVP = 0,
    // vec4, viewport size of current pass
    Output = 1,
    // vec4, viewport size of final pass
    FinalViewport = 2,
    // uint, frame count with modulo
    FrameCount = 3,
    // int, frame direction
    FrameDirection = 4,
    // float, user defined parameter, array
    FloatParameter = 5,
}

impl VariableSemantics {
    pub const fn semantics(self) -> SemanticMap<VariableSemantics, ()> {
        SemanticMap {
            semantics: self,
            index: (),
        }
    }

    pub const fn binding_type(&self) -> UniformType {
        match self {
            VariableSemantics::MVP => UniformType::MVP,
            VariableSemantics::Output => UniformType::Size,
            VariableSemantics::FinalViewport => UniformType::Size,
            VariableSemantics::FrameCount => UniformType::Unsigned,
            VariableSemantics::FrameDirection => UniformType::Signed,
            VariableSemantics::FloatParameter => UniformType::Float,
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Hash)]
#[repr(i32)]
pub enum TextureSemantics {
    Original = 0,
    Source = 1,
    OriginalHistory = 2,
    PassOutput = 3,
    PassFeedback = 4,
    User = 5,
}

impl TextureSemantics {
    pub(crate) const TEXTURE_SEMANTICS: [TextureSemantics; 6] = [
        TextureSemantics::Source,
        // originalhistory needs to come first, otherwise
        // the name lookup implementation will prioritize Original
        // when reflecting semantics.
        TextureSemantics::OriginalHistory,
        TextureSemantics::Original,
        TextureSemantics::PassOutput,
        TextureSemantics::PassFeedback,
        TextureSemantics::User,
    ];

    pub fn size_uniform_name(&self) -> &'static str {
        match self {
            TextureSemantics::Original => "OriginalSize",
            TextureSemantics::Source => "SourceSize",
            TextureSemantics::OriginalHistory => "OriginalHistorySize",
            TextureSemantics::PassOutput => "PassOutputSize",
            TextureSemantics::PassFeedback => "PassFeedbackSize",
            TextureSemantics::User => "UserSize",
        }
    }

    pub fn texture_name(&self) -> &'static str {
        match self {
            TextureSemantics::Original => "Original",
            TextureSemantics::Source => "Source",
            TextureSemantics::OriginalHistory => "OriginalHistory",
            TextureSemantics::PassOutput => "PassOutput",
            TextureSemantics::PassFeedback => "PassFeedback",
            TextureSemantics::User => "User",
        }
    }

    pub fn is_array(&self) -> bool {
        !matches!(self, TextureSemantics::Original | TextureSemantics::Source)
    }

    pub const fn semantics(self, index: usize) -> SemanticMap<TextureSemantics> {
        SemanticMap {
            semantics: self,
            index,
        }
    }
}

pub struct TypeInfo {
    pub size: u32,
    pub columns: u32,
}
pub trait ValidateTypeSemantics<T> {
    fn validate_type(&self, ty: &T) -> Option<TypeInfo>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SemanticMap<T, I = usize> {
    pub semantics: T,
    pub index: I,
}

bitflags! {
    pub struct BindingStage: u8 {
        const NONE = 0b00000000;
        const VERTEX = 0b00000001;
        const FRAGMENT = 0b00000010;
    }
}

impl BindingStage {
    pub fn clear(&mut self) {
        self.bits = 0;
    }
}

#[derive(Debug)]
pub struct UboReflection {
    pub binding: u32,
    /// Get this size of this UBO buffer.
    /// The size returned by reflection is always aligned to a 16 byte boundary.
    pub size: u32,
    pub stage_mask: BindingStage,
}

#[derive(Debug)]
pub struct PushReflection {
    /// The size returned by reflection is always aligned to a 16 byte boundary.
    pub size: u32,
    pub stage_mask: BindingStage,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MemberOffset {
    Ubo(usize),
    PushConstant(usize),
}

#[derive(Debug)]
pub struct VariableMeta {
    // this might bite us in the back because retroarch keeps separate UBO/push offsets.. eh
    pub offset: MemberOffset,
    pub components: u32,
    pub id: String,
}

#[derive(Debug)]
pub struct TextureSizeMeta {
    // this might bite us in the back because retroarch keeps separate UBO/push offsets..
    pub offset: MemberOffset,
    pub stage_mask: BindingStage,
    pub id: String,
}

#[derive(Debug)]
pub struct TextureBinding {
    pub binding: u32,
}

#[derive(Debug)]
pub struct ShaderReflection {
    pub ubo: Option<UboReflection>,
    pub push_constant: Option<PushReflection>,
    pub meta: ReflectMeta,
}

pub trait UniformMeta {
    fn offset(&self) -> MemberOffset;
    fn id(&self) -> &str;
}

impl UniformMeta for VariableMeta {
    fn offset(&self) -> MemberOffset {
        self.offset
    }

    fn id(&self) -> &str {
        &self.id
    }
}

impl UniformMeta for TextureSizeMeta {
    fn offset(&self) -> MemberOffset {
        self.offset
    }

    fn id(&self) -> &str {
        &self.id
    }
}

pub trait TextureSemanticMap<T> {
    fn get_texture_semantic(&self, name: &str) -> Option<SemanticMap<TextureSemantics>>;
}

impl TextureSemanticMap<UniformSemantic> for FxHashMap<String, UniformSemantic> {
    fn get_texture_semantic(&self, name: &str) -> Option<SemanticMap<TextureSemantics>> {
        match self.get(name) {
            None => {
                if let Some(semantics) = TextureSemantics::TEXTURE_SEMANTICS
                    .iter()
                    .find(|f| name.starts_with(f.size_uniform_name()))
                {
                    if semantics.is_array() {
                        let index = &name[semantics.size_uniform_name().len()..];
                        let Ok(index) = usize::from_str(index) else {
                            return None;
                        };
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index,
                        });
                    } else if name == semantics.size_uniform_name() {
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index: 0,
                        });
                    }
                }
                None
            }
            Some(UniformSemantic::Variable(_)) => None,
            Some(UniformSemantic::Texture(texture)) => Some(*texture),
        }
    }
}

impl TextureSemanticMap<UniformSemantic> for FxHashMap<String, SemanticMap<TextureSemantics>> {
    fn get_texture_semantic(&self, name: &str) -> Option<SemanticMap<TextureSemantics>> {
        match self.get(name) {
            None => {
                if let Some(semantics) = TextureSemantics::TEXTURE_SEMANTICS
                    .iter()
                    .find(|f| name.starts_with(f.texture_name()))
                {
                    if semantics.is_array() {
                        let index = &name[semantics.texture_name().len()..];
                        let Ok(index) = usize::from_str(index) else {return None};
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index,
                        });
                    } else if name == semantics.texture_name() {
                        return Some(SemanticMap {
                            semantics: *semantics,
                            index: 0,
                        });
                    }
                }
                None
            }
            Some(texture) => Some(*texture),
        }
    }
}

pub trait VariableSemanticMap<T> {
    fn get_variable_semantic(&self, name: &str) -> Option<SemanticMap<VariableSemantics, ()>>;
}

impl VariableSemanticMap<UniformSemantic> for FxHashMap<String, UniformSemantic> {
    fn get_variable_semantic(&self, name: &str) -> Option<SemanticMap<VariableSemantics, ()>> {
        match self.get(name) {
            // existing uniforms in the semantic map have priority
            None => match name {
                "MVP" => Some(SemanticMap {
                    semantics: VariableSemantics::MVP,
                    index: (),
                }),
                "OutputSize" => Some(SemanticMap {
                    semantics: VariableSemantics::Output,
                    index: (),
                }),
                "FinalViewportSize" => Some(SemanticMap {
                    semantics: VariableSemantics::FinalViewport,
                    index: (),
                }),
                "FrameCount" => Some(SemanticMap {
                    semantics: VariableSemantics::FrameCount,
                    index: (),
                }),
                "FrameDirection" => Some(SemanticMap {
                    semantics: VariableSemantics::FrameDirection,
                    index: (),
                }),
                _ => None,
            },
            Some(UniformSemantic::Variable(variable)) => Some(*variable),
            Some(UniformSemantic::Texture(_)) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum UniformSemantic {
    Variable(SemanticMap<VariableSemantics, ()>),
    Texture(SemanticMap<TextureSemantics>),
}

#[derive(Debug, Clone)]
pub struct ReflectSemantics {
    pub uniform_semantics: FxHashMap<String, UniformSemantic>,
    pub texture_semantics: FxHashMap<String, SemanticMap<TextureSemantics>>,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum UniformBinding {
    Parameter(String),
    SemanticVariable(VariableSemantics),
    TextureSize(SemanticMap<TextureSemantics>),
}

impl From<VariableSemantics> for UniformBinding {
    fn from(value: VariableSemantics) -> Self {
        UniformBinding::SemanticVariable(value)
    }
}

impl From<SemanticMap<TextureSemantics>> for UniformBinding {
    fn from(value: SemanticMap<TextureSemantics>) -> Self {
        UniformBinding::TextureSize(value)
    }
}
