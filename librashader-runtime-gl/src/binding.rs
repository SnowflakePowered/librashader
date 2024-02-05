use glow::HasContext;
use gl::types::GLint;
use librashader_reflect::reflect::semantics::{BindingStage, UniformMemberBlock};
use librashader_runtime::uniforms::{BindUniform, UniformScalar, UniformStorage};

#[derive(Debug, Copy, Clone)]
pub struct VariableLocation {
    pub(crate) ubo: Option<UniformLocation<Option<glow::UniformLocation>>>,
    pub(crate) push: Option<UniformLocation<Option<glow::UniformLocation>>>,
}

impl VariableLocation {
    pub fn location(&self, offset_type: UniformMemberBlock) -> Option<UniformLocation<Option<glow::UniformLocation>>> {
        match offset_type {
            UniformMemberBlock::Ubo => self.ubo,
            UniformMemberBlock::PushConstant => self.push,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UniformLocation<T> {
    pub vertex: T,
    pub fragment: T,
}

impl UniformLocation<Option<glow::UniformLocation>> {
    pub fn is_valid(&self, stage: BindingStage) -> bool {
        let mut validity = false;
        if stage.contains(BindingStage::FRAGMENT) {
            if let Some(fragment) = self.fragment {
                validity = validity || fragment.0 >= 0;
            } else {
                false
            }

        }
        if stage.contains(BindingStage::VERTEX) {
            if let Some(vertex) = self.vertex {
                validity = validity || vertex.0 >= 0;
            } else {
                false
            }
        }
        validity
    }

    pub fn bindable(&self) -> bool {
        self.is_valid(BindingStage::VERTEX | BindingStage::FRAGMENT)
    }
}

pub(crate) type GlUniformStorage = UniformStorage<GlUniformBinder, VariableLocation>;

pub trait GlUniformScalar: UniformScalar {
    const FACTORY: unsafe fn(&glow::Context, Option<&glow::UniformLocation>, Self) -> ();
}

impl GlUniformScalar for f32 {
    const FACTORY: unsafe fn(&glow::Context, Option<&glow::UniformLocation>, Self) -> () = glow::Context::uniform_1_f32;
}

impl GlUniformScalar for i32 {
    const FACTORY: unsafe fn(&glow::Context, Option<&glow::UniformLocation>, Self) -> () = glow::Context::uniform_1_i32;
}

impl GlUniformScalar for u32 {
    const FACTORY: unsafe fn(&glow::Context, Option<&glow::UniformLocation>, Self) -> () = glow::Context::uniform_1_u32;
}

pub(crate) struct GlUniformBinder;
impl<'a, T> BindUniform<(&'a glow::Context, VariableLocation), T> for GlUniformBinder
where
    T: GlUniformScalar,
{
    fn bind_uniform(block: UniformMemberBlock, value: T, ctx: (&'a glow::Context, VariableLocation)) -> Option<()> {
        let (ctx, location) = ctx;
        if let Some(location) = location.location(block)
            && location.bindable()
        {
            if location.is_valid(BindingStage::VERTEX) {
                unsafe {
                    T::FACTORY(ctx, location.vertex.as_ref(), value);
                }
            }
            if location.is_valid(BindingStage::FRAGMENT) {
                unsafe {
                    T::FACTORY(ctx, location.fragment.as_ref(), value);
                }
            }
            Some(())
        } else {
            None
        }
    }
}

impl<'a> BindUniform<(&'a glow::Context, VariableLocation), &[f32; 4]> for GlUniformBinder {
    fn bind_uniform(
        block: UniformMemberBlock,
        vec4: &[f32; 4],
        ctx: (&'a glow::Context, VariableLocation)
    ) -> Option<()> {
        let (ctx, location) = ctx;
        if let Some(location) = location.location(block)
            && location.bindable()
        {
            unsafe {
                if location.is_valid(BindingStage::VERTEX) {
                    ctx.uniform_4_f32_slice(location.vertex.as_ref(), vec4);
                }
                if location.is_valid(BindingStage::FRAGMENT) {
                    ctx.uniform_4_f32_slice(location.fragment.as_ref(), vec4);
                }
            }
            Some(())
        } else {
            None
        }
    }
}

impl<'a> BindUniform<(&'a glow::Context, VariableLocation), &[f32; 16]> for GlUniformBinder {
    fn bind_uniform(
        block: UniformMemberBlock,
        mat4: &[f32; 16],
        ctx: (&'a glow::Context, VariableLocation)
    ) -> Option<()> {
        let (ctx, location) = ctx;
        if let Some(location) = location.location(block)
            && location.bindable()
        {
            unsafe {
                if location.is_valid(BindingStage::VERTEX) {
                    ctx.uniform_matrix_4_f32_slice(location.vertex.as_ref(), false, mat4);
                }
                if location.is_valid(BindingStage::FRAGMENT) {
                    ctx.uniform_matrix_4_f32_slice(location.fragment.as_ref(), false, mat4);
                }
            }
            Some(())
        } else {
            None
        }
    }
}
