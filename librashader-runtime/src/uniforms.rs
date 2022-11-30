use librashader_reflect::reflect::semantics::MemberOffset;
use std::marker::PhantomData;

pub trait UniformScalar: Copy + bytemuck::Pod {}
impl UniformScalar for f32 {}
impl UniformScalar for i32 {}
impl UniformScalar for u32 {}

pub struct NoUniformBinder;
impl<T> BindUniform<Option<()>, T> for NoUniformBinder {
    fn bind_uniform(_: T, _: Option<()>) -> Option<()> {
        None
    }
}

pub trait BindUniform<C, T> {
    fn bind_uniform(value: T, ctx: C) -> Option<()>;
}

pub trait UniformStorageAccess {
    fn ubo_pointer(&self) -> *const u8;
    fn push_pointer(&self) -> *const u8;
}

impl<T, H> UniformStorageAccess for UniformStorage<T, H> {
    fn ubo_pointer(&self) -> *const u8 {
        self.ubo.as_ptr()
    }

    fn push_pointer(&self) -> *const u8 {
        self.push.as_ptr()
    }
}

pub struct UniformStorage<H = NoUniformBinder, C = Option<()>> {
    pub ubo: Box<[u8]>,
    pub push: Box<[u8]>,
    _h: PhantomData<H>,
    _c: PhantomData<C>,
}

impl<H, C> UniformStorage<H, C>
where
    H: BindUniform<C, f32>,
    H: BindUniform<C, u32>,
    H: BindUniform<C, i32>,
    H: for<'a> BindUniform<C, &'a [f32; 4]>,
    H: for<'a> BindUniform<C, &'a [f32; 16]>,
{
    pub fn new(ubo_size: usize, push_size: usize) -> Self {
        UniformStorage {
            ubo: vec![0u8; ubo_size].into_boxed_slice(),
            push: vec![0u8; push_size].into_boxed_slice(),
            _h: Default::default(),
            _c: Default::default(),
        }
    }

    #[inline(always)]
    fn write_scalar_inner<T: UniformScalar>(buffer: &mut [u8], value: T, ctx: C)
    where
        H: BindUniform<C, T>,
    {
        if H::bind_uniform(value, ctx).is_none() {
            let buffer = bytemuck::cast_slice_mut(buffer);
            buffer[0] = value;
        };
    }

    fn write_mat4_inner(buffer: &mut [u8], mat4: &[f32; 16], ctx: C) {
        if H::bind_uniform(mat4, ctx).is_none() {
            let mat4 = bytemuck::cast_slice(mat4);
            buffer.copy_from_slice(mat4);
        }
    }

    fn write_vec4_inner(buffer: &mut [u8], vec4: impl Into<[f32; 4]>, ctx: C) {
        let vec4 = vec4.into();
        if H::bind_uniform(&vec4, ctx).is_none() {
            let vec4 = bytemuck::cast_slice(&vec4);
            buffer.copy_from_slice(vec4);
        }
    }

    pub fn bind_mat4(&mut self, offset: MemberOffset, value: &[f32; 16], ctx: C) {
        let (buffer, offset) = match offset {
            MemberOffset::Ubo(offset) => (&mut self.ubo, offset),
            MemberOffset::PushConstant(offset) => (&mut self.push, offset),
        };
        Self::write_mat4_inner(
            &mut buffer[offset..][..16 * std::mem::size_of::<f32>()],
            value,
            ctx,
        );
    }

    pub fn bind_vec4(&mut self, offset: MemberOffset, value: impl Into<[f32; 4]>, ctx: C) {
        let (buffer, offset) = match offset {
            MemberOffset::Ubo(offset) => (&mut self.ubo, offset),
            MemberOffset::PushConstant(offset) => (&mut self.push, offset),
        };

        Self::write_vec4_inner(
            &mut buffer[offset..][..4 * std::mem::size_of::<f32>()],
            value,
            ctx,
        );
    }

    pub fn bind_scalar<T: UniformScalar>(&mut self, offset: MemberOffset, value: T, ctx: C)
    where
        H: BindUniform<C, T>,
    {
        let (buffer, offset) = match offset {
            MemberOffset::Ubo(offset) => (&mut self.ubo, offset),
            MemberOffset::PushConstant(offset) => (&mut self.push, offset),
        };

        Self::write_scalar_inner(
            &mut buffer[offset..][..std::mem::size_of::<T>()],
            value,
            ctx,
        )
    }
}
