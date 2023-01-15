use librashader_reflect::reflect::semantics::MemberOffset;
use std::marker::PhantomData;

/// A scalar value that is valid as a uniform member
pub trait UniformScalar: Copy + bytemuck::Pod {}
impl UniformScalar for f32 {}
impl UniformScalar for i32 {}
impl UniformScalar for u32 {}

/// A trait for a binder that binds the given value and context into the uniform for a shader pass.
pub trait BindUniform<C, T> {
    /// Bind the given value to the shader uniforms given the input context.
    ///
    /// A `BindUniform` implementation should not write to a backing buffer from a [`UniformStorage`](crate::uniforms::UniformStorage).
    /// If the binding is successful and no writes to a backing buffer is necessary, this function should return `Some(())`.
    /// If this function returns `None`, then the value will instead be written to the backing buffer.
    fn bind_uniform(value: T, ctx: C) -> Option<()>;
}

/// A trait to access the raw pointer to a backing uniform storage.
pub trait UniformStorageAccess {
    /// Get a pointer to the backing UBO storage. This pointer must be valid for the lifetime
    /// of the implementing struct.
    fn ubo_pointer(&self) -> *const u8;

    /// Get a pointer to the backing UBO storage. This pointer must be valid for the lifetime
    /// of the implementing struct.
    fn ubo_slice(&self) -> &[u8];

    /// Get a pointer to the backing Push Constant buffer storage.
    /// This pointer must be valid for the lifetime of the implementing struct.
    fn push_pointer(&self) -> *const u8;

    /// Get a slice to the backing Push Constant buffer storage.
    /// This pointer must be valid for the lifetime of the implementing struct.
    fn push_slice(&self) -> &[u8];
}

impl<T, H> UniformStorageAccess for UniformStorage<T, H> {
    fn ubo_pointer(&self) -> *const u8 {
        self.ubo.as_ptr()
    }

    fn ubo_slice(&self) -> &[u8] {
        &self.ubo
    }

    fn push_pointer(&self) -> *const u8 {
        self.push.as_ptr()
    }

    fn push_slice(&self) -> &[u8] {
        &self.push
    }
}

/// A uniform binder that always returns `None`, and does not do any binding of uniforms.
/// All uniform data is thus written into the backing buffer storage.
pub struct NoUniformBinder;
impl<T> BindUniform<Option<()>, T> for NoUniformBinder {
    fn bind_uniform(_: T, _: Option<()>) -> Option<()> {
        None
    }
}

/// A helper to bind uniform variables to UBO or Push Constant Buffers.
pub struct UniformStorage<H = NoUniformBinder, C = Option<()>> {
    ubo: Box<[u8]>,
    push: Box<[u8]>,
    _h: PhantomData<H>,
    _c: PhantomData<C>,
}

impl<H, C> UniformStorage<H, C> {
    /// Create a new `UniformStorage` with the given size for UBO and Push Constant Buffer sizes.
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

    /// Bind a scalar to the given offset.
    #[inline(always)]
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

impl<H, C> UniformStorage<H, C>
where
    H: for<'a> BindUniform<C, &'a [f32; 4]>,
{
    #[inline(always)]
    fn write_vec4_inner(buffer: &mut [u8], vec4: impl Into<[f32; 4]>, ctx: C) {
        let vec4 = vec4.into();
        if H::bind_uniform(&vec4, ctx).is_none() {
            let vec4 = bytemuck::cast_slice(&vec4);
            buffer.copy_from_slice(vec4);
        }
    }
    /// Bind a `vec4` to the given offset.
    #[inline(always)]
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
}

impl<H, C> UniformStorage<H, C>
where
    H: for<'a> BindUniform<C, &'a [f32; 16]>,
{
    #[inline(always)]
    fn write_mat4_inner(buffer: &mut [u8], mat4: &[f32; 16], ctx: C) {
        if H::bind_uniform(mat4, ctx).is_none() {
            let mat4 = bytemuck::cast_slice(mat4);
            buffer.copy_from_slice(mat4);
        }
    }

    /// Bind a `mat4` to the given offset.
    #[inline(always)]
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
}
