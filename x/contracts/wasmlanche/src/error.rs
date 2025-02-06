use std::fmt;
use std::io;
use borsh::io::Error as BorshError;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    BorshError(io::Error),
    StateError(String),
    ExternalCallError(String),
    Custom(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::BorshError(e) => write!(f, "Borsh error: {}", e),
            Error::StateError(s) => write!(f, "State error: {}", s),
            Error::ExternalCallError(s) => write!(f, "External call error: {}", s),
            Error::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::Custom(error)
    }
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error::Custom(error.to_string())
    }
}

impl From<crate::state::Error> for Error {
    fn from(err: crate::state::Error) -> Self {
        match err {
            crate::state::Error::BorshError(e) => Error::BorshError(e),
            crate::state::Error::StateError(e) => Error::StateError(e),
        }
    }
}
