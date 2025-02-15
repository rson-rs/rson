use std::fmt;

use serde::de::Error as DeError;
use thiserror::Error;

pub type RsonDeResult<T> = Result<T, RsonDeError>;

#[derive(Debug, Error)]
pub enum RsonDeError {
    #[error("Failed to parse input as statements")]
    NoStatements,

    #[error("Failed to find return expression")]
    ReturnExprNotFound,

    #[error("Failed to parse: {0}")]
    Parse(String),

    #[error("Failed to parse: {0}")]
    Syn(#[from] syn::Error),

    #[error("Failed to deserialize: {0}")]
    Serde(String),
}

impl DeError for RsonDeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Serde(msg.to_string())
    }
}
