#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use borsh::{BorshDeserialize, BorshSerialize, maybestd};

#[cfg(not(target_arch = "wasm32"))]
use thiserror::Error;

#[cfg(not(target_arch = "wasm32"))]
use async_trait::async_trait;

#[cfg(not(target_arch = "wasm32"))]
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
    /// IO error
    #[error("IO error")]
    Io,
    /// Invalid argument provided
    #[error("Invalid argument")]
    InvalidArgument,
    /// Invalid data format
    #[error("Invalid data")]
    InvalidData,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub enum Error {
    /// Error during serialization/deserialization
    SerializationFailed,
    /// Error accessing storage
    StorageFailed,
    /// State error
    State(String),
    /// Serialization error
    Serialization(String),
    /// IO error
    Io,
    /// Invalid argument provided
    InvalidArgument,
    /// Invalid data format
    InvalidData,
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::State(msg)
    }
}

impl From<maybestd::io::Error> for Error {
    fn from(_: maybestd::io::Error) -> Self {
        Error::Io
    }
}

/// Trait for types that can be used as state keys
pub trait StateKey: Default {
    /// Get the key bytes for this state type
    fn key(&self) -> Vec<u8>;
    
    /// Get the key bytes for this state type
    fn key_static() -> Vec<u8> {
        Self::default().key()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait StateAccess {
    async fn store_state<S: BorshSerialize + StateKey + Send + Sync>(&mut self, state: &S) -> Result<(), Error>;
    async fn get_state<S: BorshDeserialize + StateKey + Default + Send + Sync>(&self) -> Result<Option<S>, Error>;
    async fn delete_state<S: BorshDeserialize + StateKey + Default + Send + Sync>(&mut self) -> Result<Option<S>, Error>;
}

#[cfg(target_arch = "wasm32")]
pub trait StateAccess {
    fn store_state<S: BorshSerialize + StateKey>(&mut self, state: &S) -> Result<(), Error>;
    fn get_state<S: BorshDeserialize + StateKey + Default>(&self) -> Result<Option<S>, Error>;
    fn delete_state<S: BorshDeserialize + StateKey + Default>(&mut self) -> Result<Option<S>, Error>;
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

    #[cfg(not(target_arch = "wasm32"))]
    #[async_trait]
    impl StateAccess for TestStateAccess {
        async fn store_state<S: BorshSerialize + StateKey + Send + Sync>(&mut self, state: &S) -> Result<(), Error> {
            let bytes = BorshSerialize::try_to_vec(state)
                .map_err(|err| Error::SerializationFailed)?;
            let mut state_guard = self.state.write().await;
            *state_guard = Some(bytes);
            Ok(())
        }

        async fn get_state<S: BorshDeserialize + StateKey + Default + Send + Sync>(&self) -> Result<Option<S>, Error> {
            let state_guard = self.state.read().await;
            match &*state_guard {
                Some(bytes) => {
                    let state = BorshDeserialize::try_from_slice(bytes)
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
                    let state = BorshDeserialize::try_from_slice(&bytes)
                        .map_err(|err| Error::SerializationFailed)?;
                    Ok(Some(state))
                }
                None => Ok(None),
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    impl StateAccess for TestStateAccess {
        fn store_state<S: BorshSerialize + StateKey>(&mut self, state: &S) -> Result<(), Error> {
            let bytes = BorshSerialize::try_to_vec(state)
                .map_err(|err| Error::SerializationFailed)?;
            let mut state_guard = self.state.write().unwrap();
            *state_guard = Some(bytes);
            Ok(())
        }

        fn get_state<S: BorshDeserialize + StateKey + Default>(&self) -> Result<Option<S>, Error> {
            let state_guard = self.state.read().unwrap();
            match &*state_guard {
                Some(bytes) => {
                    let state = BorshDeserialize::try_from_slice(bytes)
                        .map_err(|err| Error::SerializationFailed)?;
                    Ok(Some(state))
                }
                None => Ok(None),
            }
        }

        fn delete_state<S: BorshDeserialize + StateKey + Default>(&mut self) -> Result<Option<S>, Error> {
            let mut state_guard = self.state.write().unwrap();
            match state_guard.take() {
                Some(bytes) => {
                    let state = BorshDeserialize::try_from_slice(&bytes)
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
