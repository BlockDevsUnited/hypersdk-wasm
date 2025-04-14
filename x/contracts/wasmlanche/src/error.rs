// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use core::fmt;
use core::error;
use borsh::{BorshSerialize, BorshDeserialize};
use borsh::maybestd::io;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::string::String;

#[cfg(feature = "std")]
use std::string::String;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum Error {
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
    InvalidChain(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::State(msg) => write!(f, "State error: {}", msg),
            Error::Event(msg) => write!(f, "Event error: {}", msg),
            Error::Gas(msg) => write!(f, "Gas error: {}", msg),
            Error::Memory(msg) => write!(f, "Memory error: {}", msg),
            Error::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            Error::Contract(msg) => write!(f, "Contract error: {}", msg),
            Error::Crypto(msg) => write!(f, "Crypto error: {}", msg),
            Error::TooExpensive(msg) => write!(f, "Too expensive: {}", msg),
            Error::Unknown(msg) => write!(f, "Unknown error: {}", msg),
            Error::NameTooLong(msg) => write!(f, "Name too long: {}", msg),
            Error::DataTooLarge(msg) => write!(f, "Data too large: {}", msg),
            Error::TooManyEvents(msg) => write!(f, "Too many events: {}", msg),
            Error::InvalidChain(msg) => write!(f, "Invalid chain: {}", msg),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<io::Error> for Error {
    fn from(_err: io::Error) -> Self {
        Error::Unknown(String::from("IO error occurred"))
    }
}

// Helper method for handling borsh::maybestd::io::Error
impl Error {
    pub fn from_borsh_io(_err: borsh::maybestd::io::Error) -> Self {
        Error::Serialization(String::from("Failed to serialize/deserialize"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test error");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::Unknown(_)));

        let borsh_err = io::Error::new(io::ErrorKind::Other, "test error");
        let err = Error::from_borsh_io(borsh_err);
        assert!(matches!(err, Error::Serialization(_)));
    }
}

// Export EventError as a type alias for backward compatibility
pub type EventError = Error;
