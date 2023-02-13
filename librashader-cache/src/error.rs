use std::error::Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("serde error")]
    SerdeError,
    #[error("unknown error")]
    UnknownError(#[from] Box<dyn Error>),
}
