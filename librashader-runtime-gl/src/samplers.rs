use gl::types::{GLenum, GLint, GLuint};
use librashader_common::{FilterMode, WrapMode};
use rustc_hash::FxHashMap;

pub struct SamplerSet {
    // todo: may need to deal with differences in mip filter.
    samplers: FxHashMap<(WrapMode, FilterMode, FilterMode), GLuint>,
}

impl SamplerSet {
    pub fn get(&self, wrap: WrapMode, filter: FilterMode, mip: FilterMode) -> GLuint {
        // eprintln!("{wrap}, {filter}, {mip}");
        *self.samplers.get(&(wrap, filter, mip)).unwrap()
    }

    fn make_sampler(sampler: GLuint, wrap: WrapMode, filter: FilterMode, mip: FilterMode) {
        unsafe {
            gl::SamplerParameteri(sampler, gl::TEXTURE_WRAP_S, GLenum::from(wrap) as GLint);
            gl::SamplerParameteri(sampler, gl::TEXTURE_WRAP_T, GLenum::from(wrap) as GLint);
            gl::SamplerParameteri(
                sampler,
                gl::TEXTURE_MAG_FILTER,
                GLenum::from(filter) as GLint,
            );

            gl::SamplerParameteri(sampler, gl::TEXTURE_MIN_FILTER, filter.gl_mip(mip) as GLint);
        }
    }

    pub fn new() -> SamplerSet {
        let mut samplers = FxHashMap::default();
        let wrap_modes = &[
            WrapMode::ClampToBorder,
            WrapMode::ClampToEdge,
            WrapMode::Repeat,
            WrapMode::MirroredRepeat,
        ];
        for wrap_mode in wrap_modes {
            unsafe {
                let mut linear_linear = 0;
                let mut linear_nearest = 0;

                let mut nearest_nearest = 0;
                let mut nearest_linear = 0;
                gl::GenSamplers(1, &mut linear_linear);
                gl::GenSamplers(1, &mut linear_nearest);
                gl::GenSamplers(1, &mut nearest_linear);
                gl::GenSamplers(1, &mut nearest_nearest);

                SamplerSet::make_sampler(
                    linear_linear,
                    *wrap_mode,
                    FilterMode::Linear,
                    FilterMode::Linear,
                );
                SamplerSet::make_sampler(
                    linear_nearest,
                    *wrap_mode,
                    FilterMode::Linear,
                    FilterMode::Nearest,
                );
                SamplerSet::make_sampler(
                    nearest_linear,
                    *wrap_mode,
                    FilterMode::Nearest,
                    FilterMode::Linear,
                );
                SamplerSet::make_sampler(
                    nearest_nearest,
                    *wrap_mode,
                    FilterMode::Nearest,
                    FilterMode::Nearest,
                );

                samplers.insert(
                    (*wrap_mode, FilterMode::Linear, FilterMode::Linear),
                    linear_linear,
                );
                samplers.insert(
                    (*wrap_mode, FilterMode::Linear, FilterMode::Nearest),
                    linear_nearest,
                );

                samplers.insert(
                    (*wrap_mode, FilterMode::Nearest, FilterMode::Nearest),
                    nearest_nearest,
                );
                samplers.insert(
                    (*wrap_mode, FilterMode::Nearest, FilterMode::Linear),
                    nearest_linear,
                );
            }
        }

        SamplerSet { samplers }
    }
}
