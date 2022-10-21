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
}
