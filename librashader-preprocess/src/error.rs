use std::convert::Infallible;
use std::path::PathBuf;
use thiserror::Error;
use librashader::ShaderParameter;

#[derive(Error, Debug)]
pub enum PreprocessError {
    #[error("the version header was missing")]
    MissingVersionHeader,
    #[error("the file was not found during resolution")]
    IOError(PathBuf, std::io::Error),
    #[error("unexpected end of file")]
    UnexpectedEof,
    #[error("unexpected end of line")]
    UnexpectedEol(usize),
    #[error("error parsing pragma")]
    PragmaParseError(String),
    #[error("duplicate parameter but arguments do not match")]
    DuplicateParameterError(String),
    #[error("shader format is unknown or not found")]
    UnknownShaderFormat,
    #[error("tried to declare shader format twice")]
    DuplicateShaderFormat,

}

impl From<Infallible> for PreprocessError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}