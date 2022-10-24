use crate::error::ShaderReflectError;

pub const BASE_SEMANTICS_COUNT: usize = 5;
pub const MAX_BINDINGS_COUNT: u32 = 16;

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

#[repr(i32)]
pub enum TextureSemantics {
    Original = 0,
    Source = 1,
    OriginalHistory = 2,
    PassOutput = 3,
    PassFeedback = 4,
    User = 5,
}

pub struct BufferReflection {
    pub binding: Option<u32>,
    pub size: usize,
    pub stage_mask: u32,
}

pub struct ShaderReflection {
    pub ubo: Option<BufferReflection>,
    pub push_constant: Option<BufferReflection>,
}
