use std::{fmt, io};

use serde::ser::Error as SerError;
use thiserror::Error;

pub type RsonSerResult<T> = Result<T, RsonSerError>;

#[derive(Debug, Error)]
pub enum RsonSerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to serialize: {0}")]
    Serde(String),
}

impl SerError for RsonSerError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Serde(msg.to_string())
    }
}
