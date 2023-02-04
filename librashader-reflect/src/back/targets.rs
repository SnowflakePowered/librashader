/// Marker trait for shader compiler targets.
pub trait OutputTarget {
    /// The output format for the target.
    type Output;
}

/// Shader compiler target for GLSL.
pub struct GLSL;
/// Shader compiler target for HLSL.
pub struct HLSL;
/// Shader compiler target for SPIR-V.
pub struct SPIRV;
/// Shader compiler target for MSL.
pub struct MSL;
/// Shader compiler target for DXIL.
///
/// The resulting DXIL object is always unvalidated and
/// must be validated using platform APIs before usage.
pub struct DXIL;

impl OutputTarget for GLSL {
    type Output = String;
}
impl OutputTarget for HLSL {
    type Output = String;
}
impl OutputTarget for SPIRV {
    type Output = Vec<u32>;
}

mod test {
    use crate::back::targets::GLSL;
    use crate::back::FromCompilation;
    use crate::front::GlslangCompilation;
    #[allow(dead_code)]
    pub fn test_compile(value: GlslangCompilation) {
        let _x = GLSL::from_compilation(value).unwrap();
    }
}
