use crate::parse::Span;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParsePresetError {
    #[error("shader preset lexing error")]
    LexerError { offset: usize, row: u32, col: usize },
    #[error("shader preset parse error")]
    ParserError {
        offset: usize,
        row: u32,
        col: usize,
        kind: ParseErrorKind,
    },
    #[error("invalid scale type")]
    InvalidScaleType(String),
    #[error("exceeded maximum reference depth (16)")]
    ExceededReferenceDepth,
    #[error("shader presents must be resolved against an absolute path")]
    RootPathWasNotAbsolute,
    #[error("the file was not found during resolution")]
    IOError(PathBuf, std::io::Error),
    #[error("expected utf8 bytes but got invalid utf8")]
    Utf8Error(Vec<u8>),
}

#[derive(Debug)]
pub enum ParseErrorKind {
    Index(&'static str),
    Int,
    UnsignedInt,
    Float,
    Bool,
}
