use std::error::Error as StdError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::{fmt, io};

use serde::de;

use crate::parse::Position;

/// Deserialization result.
pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    IoError(String),
    Message(String),
    Parser(ParseError, Position),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    Eof,
    ExpectedArray,
    ExpectedArrayEnd,
    ExpectedBoolean,
    ExpectedComma,
    ExpectedEnum,
    ExpectedChar,
    ExpectedFloat,
    ExpectedInteger,
    ExpectedOption,
    ExpectedOptionEnd,
    ExpectedMap,
    ExpectedMapColon,
    ExpectedMapEnd,
    ExpectedStruct,
    ExpectedStructEnd,
    ExpectedUnit,
    ExpectedStructName,
    ExpectedString,
    ExpectedStringEnd,
    ExpectedIdentifier,

    InvalidEscape,

    UnexpectedByte(char),

    Utf8Error(Utf8Error),
    TrailingCharacters,

    #[doc(hidden)]
    __NonExhaustive,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IoError(ref s) => write!(f, "{}", s),
            Error::Message(ref s) => write!(f, "{}", s),
            Error::Parser(_, pos) => write!(f, "{}: {}", pos, self.to_string()),
        }
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl StdError for Error {}

impl From<Utf8Error> for ParseError {
    fn from(e: Utf8Error) -> Self {
        ParseError::Utf8Error(e)
    }
}

impl From<FromUtf8Error> for ParseError {
    fn from(e: FromUtf8Error) -> Self {
        ParseError::Utf8Error(e.utf8_error())
    }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error::Parser(ParseError::Utf8Error(e), Position { line: 0, col: 0 })
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e.to_string())
    }
}
