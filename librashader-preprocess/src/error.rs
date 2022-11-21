use crate::ShaderParameter;
use std::convert::Infallible;
use std::path::PathBuf;
use thiserror::Error;

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
    #[error("duplicate pragma found")]
    DuplicatePragmaError(String),
    #[error("shader format is unknown or not found")]
    UnknownShaderFormat,
    #[error("stage must be either vertex or fragment")]
    InvalidStage,
}

impl From<Infallible> for PreprocessError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
