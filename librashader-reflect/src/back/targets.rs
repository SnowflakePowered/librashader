pub trait OutputTarget {
    type Output;
}

pub struct GLSL;
pub struct HLSL;
pub struct SPIRV;
pub struct MSL;

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
    use crate::back::FromCompilation;
    use crate::back::targets::GLSL;
    use crate::front::shaderc::GlslangCompilation;
    #[allow(dead_code)]
    pub fn test_compile(value: GlslangCompilation) {
        let _x = GLSL::from_compilation(value).unwrap();
    }
}
