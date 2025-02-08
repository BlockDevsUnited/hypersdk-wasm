// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::io;
use borsh::maybestd::io as borsh_io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    State(String),
    Event(String),
    Gas(String),
    Memory(String),
    Serialization(String),
    Contract(String),
    Crypto(String),
    TooExpensive(String),
    Unknown(String),
    NameTooLong(String),
    DataTooLarge(String),
    TooManyEvents(String),
    InvalidSignature(String),
    InvalidNonce(String),
    Unauthorized(String),
    MaxDepthExceeded(String),
    InvalidProtocolVersion(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::State(e) => write!(f, "State error: {}", e),
            Error::Event(e) => write!(f, "Event error: {}", e),
            Error::Gas(e) => write!(f, "Gas error: {}", e),
            Error::Memory(e) => write!(f, "Memory error: {}", e),
            Error::Serialization(e) => write!(f, "Serialization error: {}", e),
            Error::Contract(e) => write!(f, "Contract error: {}", e),
            Error::Crypto(e) => write!(f, "Crypto error: {}", e),
            Error::TooExpensive(e) => write!(f, "Too expensive error: {}", e),
            Error::Unknown(e) => write!(f, "Unknown error: {}", e),
            Error::NameTooLong(e) => write!(f, "Name too long error: {}", e),
            Error::DataTooLarge(e) => write!(f, "Data too large error: {}", e),
            Error::TooManyEvents(e) => write!(f, "Too many events error: {}", e),
            Error::InvalidSignature(e) => write!(f, "Invalid signature error: {}", e),
            Error::InvalidNonce(e) => write!(f, "Invalid nonce error: {}", e),
            Error::Unauthorized(e) => write!(f, "Unauthorized error: {}", e),
            Error::MaxDepthExceeded(e) => write!(f, "Max depth exceeded error: {}", e),
            Error::InvalidProtocolVersion(e) => write!(f, "Invalid protocol version error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

// Instead of implementing From for borsh_io::Error, provide a helper method
impl Error {
    pub fn from_borsh_io(err: borsh_io::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test error");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::Io(_)));

        let borsh_err = borsh_io::Error::new(
            borsh_io::ErrorKind::Other,
            "test error",
        );
        let err = Error::from_borsh_io(borsh_err);
        assert!(matches!(err, Error::Serialization(_)));
    }
}

// Export EventError as a type alias for backward compatibility
pub type EventError = Error;
