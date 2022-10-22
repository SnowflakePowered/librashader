mod error;
mod include;
mod pragma;

pub use error::*;

#[cfg(test)]
mod test {
    use crate::include::read_source;
    #[test]
    pub fn preprocess_file() {
        let result =
            read_source("../test/slang-shaders/blurs/shaders/royale/blur3x3-last-pass.slang")
                .unwrap();
        eprintln!("{result}")
    }
}
