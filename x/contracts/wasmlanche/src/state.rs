// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::string::String;
#[cfg(feature = "std")]
use std::vec::Vec;

use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use thiserror::Error;

#[derive(Debug)]
pub enum Error {
    State(String),
    Serialization(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::State(msg) => write!(f, "State error: {}", msg),
            Error::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<borsh::maybestd::io::Error> for Error {
    fn from(err: borsh::maybestd::io::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl Error {
    pub fn from_io(err: std::io::Error) -> Self {
        Error::State(err.to_string())
    }
}

pub trait StateKey {
    fn get_key() -> Vec<u8>;
}

#[async_trait::async_trait]
pub trait StateAccess {
    async fn store_state<S: BorshSerialize + StateKey + Send + Sync>(&mut self, state: &S) -> Result<(), Error>;
    async fn get_state<S: BorshDeserialize + StateKey + Send + Sync>(&self) -> Result<Option<S>, Error>;
    async fn delete_state<S: BorshDeserialize + StateKey + Send + Sync>(&mut self) -> Result<Option<S>, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(BorshSerialize, BorshDeserialize)]
    struct TestState {
        value: String,
    }

    impl StateKey for TestState {
        fn get_key() -> Vec<u8> {
            b"test_state".to_vec()
        }
    }

    struct TestStateAccess {
        state: Arc<RwLock<Option<Vec<u8>>>>,
    }

    #[async_trait::async_trait]
    impl StateAccess for TestStateAccess {
        async fn store_state<S: BorshSerialize + StateKey + Send + Sync>(&mut self, state: &S) -> Result<(), Error> {
            let bytes = borsh::BorshSerialize::try_to_vec(state)
                .map_err(|err| Error::Serialization(err.to_string()))?;
            let mut state_guard = self.state.write().await;
            *state_guard = Some(bytes);
            Ok(())
        }

        async fn get_state<S: BorshDeserialize + StateKey + Send + Sync>(&self) -> Result<Option<S>, Error> {
            let state_guard = self.state.read().await;
            match &*state_guard {
                Some(bytes) => {
                    let state = borsh::BorshDeserialize::try_from_slice(bytes)
                        .map_err(|err| Error::Serialization(err.to_string()))?;
                    Ok(Some(state))
                }
                None => Ok(None),
            }
        }

        async fn delete_state<S: BorshDeserialize + StateKey + Send + Sync>(&mut self) -> Result<Option<S>, Error> {
            let mut state_guard = self.state.write().await;
            match state_guard.take() {
                Some(bytes) => {
                    let state = borsh::BorshDeserialize::try_from_slice(&bytes)
                        .map_err(|err| Error::Serialization(err.to_string()))?;
                    Ok(Some(state))
                }
                None => Ok(None),
            }
        }
    }

    #[tokio::test]
    async fn test_state_operations() {
        let state_access = TestStateAccess {
            state: Arc::new(RwLock::new(None)),
        };

        let test_state = TestState {
            value: "test".to_string(),
        };

        let mut state_access = state_access;

        // Test store_state
        state_access.store_state(&test_state).await.unwrap();

        // Test get_state
        let retrieved: Option<TestState> = state_access.get_state().await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "test");

        // Test delete_state
        let deleted: Option<TestState> = state_access.delete_state().await.unwrap();
        assert!(deleted.is_some());
        assert_eq!(deleted.unwrap().value, "test");

        // Verify state is deleted
        let retrieved: Option<TestState> = state_access.get_state().await.unwrap();
        assert!(retrieved.is_none());
    }
}
