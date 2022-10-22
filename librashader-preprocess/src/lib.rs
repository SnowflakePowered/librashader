mod error;
mod include;
mod pragma;

pub use error::*;

#[cfg(test)]
mod test {
    use crate::include::read_source;
    use crate::pragma;

    #[test]
    pub fn preprocess_file() {
        let result =
            read_source("../test/slang-shaders/blurs/shaders/royale/blur3x3-last-pass.slang")
                .unwrap();
        eprintln!("{result}")
    }

    #[test]
    pub fn get_param_pragmas() {
        let result =
            read_source("../test/slang-shaders/crt/shaders/crt-maximus-royale/src/ntsc_pass1.slang")
                .unwrap();

        let params = pragma::parse_pragma_meta(result)
            .unwrap();
        eprintln!("{params:?}")
    }
}
