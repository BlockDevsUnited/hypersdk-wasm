#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use thiserror::Error;

/// Error type for state operations
#[derive(Debug, Error)]
pub enum Error {
    /// Error during serialization/deserialization
    #[error("Serialization failed")]
    SerializationFailed,
    /// Error accessing storage
    #[error("Storage operation failed")]
    StorageFailed,
    /// State error
    #[error("State error: {0}")]
    State(String),
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<borsh::maybestd::io::Error> for Error {
    fn from(_: borsh::maybestd::io::Error) -> Self {
        Error::SerializationFailed
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::State(msg)
    }
}

/// Trait for types that can be used as state keys
pub trait StateKey: Default {
    /// Get the key bytes for this state type
    fn key(&self) -> Vec<u8>;
}

#[async_trait::async_trait]
pub trait StateAccess {
    async fn store_state<S: BorshSerialize + StateKey + Send + Sync>(&mut self, state: &S) -> Result<(), Error>;
    async fn get_state<S: BorshDeserialize + StateKey + Default + Send + Sync>(&self) -> Result<Option<S>, Error>;
    async fn delete_state<S: BorshDeserialize + StateKey + Default + Send + Sync>(&mut self) -> Result<Option<S>, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(BorshSerialize, BorshDeserialize, Default)]
    struct TestState {
        value: String,
    }

    impl StateKey for TestState {
        fn key(&self) -> Vec<u8> {
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
                .map_err(|err| Error::SerializationFailed)?;
            let mut state_guard = self.state.write().await;
            *state_guard = Some(bytes);
            Ok(())
        }

        async fn get_state<S: BorshDeserialize + StateKey + Default + Send + Sync>(&self) -> Result<Option<S>, Error> {
            let state_guard = self.state.read().await;
            match &*state_guard {
                Some(bytes) => {
                    let state = borsh::BorshDeserialize::try_from_slice(bytes)
                        .map_err(|err| Error::SerializationFailed)?;
                    Ok(Some(state))
                }
                None => Ok(None),
            }
        }

        async fn delete_state<S: BorshDeserialize + StateKey + Default + Send + Sync>(&mut self) -> Result<Option<S>, Error> {
            let mut state_guard = self.state.write().await;
            match state_guard.take() {
                Some(bytes) => {
                    let state = borsh::BorshDeserialize::try_from_slice(&bytes)
                        .map_err(|err| Error::SerializationFailed)?;
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
