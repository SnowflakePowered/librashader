use gl::types::GLint;
use librashader_reflect::reflect::semantics::BindingStage;
use librashader_reflect::reflect::uniforms::{BindUniform, UniformBuffer, UniformScalar};

#[derive(Debug)]
pub enum VariableLocation {
    Ubo(UniformLocation<GLint>),
    Push(UniformLocation<GLint>),
}

impl VariableLocation {
    pub fn location(&self) -> UniformLocation<GLint> {
        match self {
            VariableLocation::Ubo(l) | VariableLocation::Push(l) => *l,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UniformLocation<T> {
    pub vertex: T,
    pub fragment: T,
}

impl UniformLocation<GLint> {
    pub fn is_valid(&self, stage: BindingStage) -> bool {
        let mut validity = false;
        if stage.contains(BindingStage::FRAGMENT) {
            validity = validity || self.fragment >= 0;
        }
        if stage.contains(BindingStage::VERTEX) {
            validity = validity || self.vertex >= 0;
        }
        validity
    }
}

pub(crate) type BufferStorage = UniformBuffer<GlUniformBinder, UniformLocation<GLint>>;


pub trait GlUniformScalar: UniformScalar {
    const FACTORY: unsafe fn(GLint, Self) -> ();
}

impl GlUniformScalar for f32 {
    const FACTORY: unsafe fn(GLint, Self) -> () = gl::Uniform1f;
}

impl GlUniformScalar for i32 {
    const FACTORY: unsafe fn(GLint, Self) -> () = gl::Uniform1i;
}

impl GlUniformScalar for u32 {
    const FACTORY: unsafe fn(GLint, Self) -> () = gl::Uniform1ui;
}

pub(crate) struct GlUniformBinder;
impl<T> BindUniform<UniformLocation<GLint>, T> for GlUniformBinder
    where T: GlUniformScalar
{
    fn bind_uniform(value: T, location: UniformLocation<GLint>) -> Option<()> {
        if location.is_valid(BindingStage::VERTEX | BindingStage::FRAGMENT) {
            unsafe {
                if location.is_valid(BindingStage::VERTEX) {
                    T::FACTORY(location.vertex, value);
                }
                if location.is_valid(BindingStage::FRAGMENT) {
                    T::FACTORY(location.fragment, value);
                }
            }
            Some(())
        } else {
            None
        }
    }
}

impl BindUniform<UniformLocation<GLint>, &[f32; 4]> for GlUniformBinder {
    fn bind_uniform(vec4: &[f32; 4], location: UniformLocation<GLint>) -> Option<()> {
        if location.is_valid(BindingStage::VERTEX | BindingStage::FRAGMENT) {
            unsafe {
                if location.is_valid(BindingStage::VERTEX) {
                    gl::Uniform4fv(location.vertex, 1, vec4.as_ptr());
                }
                if location.is_valid(BindingStage::FRAGMENT) {
                    gl::Uniform4fv(location.fragment, 1, vec4.as_ptr());
                }
            }
            Some(())
        } else {
            None
        }
    }
}

impl BindUniform<UniformLocation<GLint>, &[f32; 16]> for GlUniformBinder {
    fn bind_uniform(mat4: &[f32; 16], location: UniformLocation<GLint>) -> Option<()> {
        if location.is_valid(BindingStage::VERTEX | BindingStage::FRAGMENT) {
            unsafe {
                if location.is_valid(BindingStage::VERTEX) {
                    gl::UniformMatrix4fv(location.vertex, 1, gl::FALSE, mat4.as_ptr());
                }
                if location.is_valid(BindingStage::FRAGMENT) {
                    gl::UniformMatrix4fv(location.fragment, 1, gl::FALSE, mat4.as_ptr());
                }
            }
            Some(())
        }else {
            None
        }
    }
}