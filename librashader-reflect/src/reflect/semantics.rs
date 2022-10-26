use crate::error::ShaderReflectError;
use bitflags::bitflags;
use crate::reflect::ReflectMeta;

pub const BASE_SEMANTICS_COUNT: usize = 5;
pub const MAX_BINDINGS_COUNT: u32 = 16;
pub const MAX_PUSH_BUFFER_SIZE: u32 = 128;

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

pub struct TypeInfo {
    pub size: u32,
    pub columns: u32,
}
pub trait ValidateTypeSemantics<T> {
    fn validate_type(&self, ty: &T) -> Option<TypeInfo>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SemanticMap<T> {
    pub(crate) semantics: T,
    pub(crate) index: u32
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
    pub size: u32,
    pub stage_mask: BindingStage,
}

#[derive(Debug)]
pub struct PushReflection {
    pub size: u32,
    pub stage_mask: BindingStage,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MemberOffset {
    Ubo(usize),
    PushConstant(usize)
}

#[derive(Debug)]
pub struct VariableMeta {
    // this might bite us in the back because retroarch keeps separate UBO/push offsets.. eh
    pub offset: MemberOffset,
    pub components: u32
}

#[derive(Debug)]
pub struct TextureMeta {
    // this might bite us in the back because retroarch keeps separate UBO/push offsets..
    pub offset: MemberOffset,
    pub stage_mask: BindingStage,
    pub texture: bool
}

#[derive(Debug)]
pub struct ShaderReflection {
    pub ubo: Option<UboReflection>,
    pub push_constant: Option<PushReflection>,
    pub meta: ReflectMeta
}
