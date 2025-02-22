#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};

use borsh::{BorshDeserialize, BorshSerialize};
use spin::RwLock;

use crate::{
    events::Event,
    host::{Host, HostImpl, HostState},
    state::{Error as StateError, StateKey},
    types::WasmlAddress,
};

/// Execution context for a contract
#[cfg(not(target_arch = "wasm32"))]
pub struct Context {
    pub actor: WasmlAddress,
    host: Box<dyn Host>,
    state: RwLock<HostState>,
}

#[cfg(target_arch = "wasm32")]
pub struct Context {
    pub actor: WasmlAddress,
    state: RwLock<()>,
}

impl Context {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        Self::with_actor(WasmlAddress::default())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Self {
        Self::with_actor(WasmlAddress::default())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_actor(actor: WasmlAddress) -> Self {
        Self {
            actor,
            host: Box::new(HostImpl::new(actor)),
            state: RwLock::new(HostState::default()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn with_actor(actor: WasmlAddress) -> Self {
        Self {
            actor,
            state: RwLock::new(()),
        }
    }

    /// Store a value in state
    pub fn store<T: BorshSerialize>(&mut self, key: &[u8], value: &T) -> Result<(), StateError> {
        let bytes = value.try_to_vec()?;
        self.store_by_key(key, bytes)
    }

    /// Store raw bytes in state
    pub fn store_by_key(&mut self, key: &[u8], value: Vec<u8>) -> Result<(), StateError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut state = self.state.write();
            state.store(key, value.clone());
            self.host.emit_event(Event::StateChange {
                key: key.to_vec(),
                value,
            }).map_err(|_| StateError::StorageFailed)?;
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Call host function to store state
            extern "C" {
                fn store_state(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
            }
            unsafe {
                let result = store_state(
                    key.as_ptr(),
                    key.len(),
                    value.as_ptr(),
                    value.len()
                );
                if result == 0 {
                    Ok(())
                } else {
                    Err(StateError::StorageFailed)
                }
            }
        }
    }

    /// Get a value from state
    pub fn get<T: BorshDeserialize>(&self, key: &[u8]) -> Result<Option<T>, StateError> {
        if let Some(bytes) = self.get_by_key(key)? {
            Ok(Some(T::try_from_slice(&bytes)?))
        } else {
            Ok(None)
        }
    }

    /// Get raw bytes from state
    pub fn get_by_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StateError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let state = self.state.read();
            Ok(state.get(key).cloned())
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Call host function to get state
            extern "C" {
                fn get_state(key_ptr: *const u8, key_len: usize) -> i32;
                fn get_value(value_ptr: *mut u8, value_len: usize) -> i32;
            }
            unsafe {
                let result = get_state(key.as_ptr(), key.len());
                if result < 0 {
                    return Err(StateError::StorageFailed);
                }
                if result == 0 {
                    return Ok(None);
                }
                let mut value = Vec::with_capacity(result as usize);
                let result = get_value(value.as_mut_ptr(), result as usize);
                if result < 0 {
                    return Err(StateError::StorageFailed);
                }
                value.set_len(result as usize);
                Ok(Some(value))
            }
        }
    }

    /// Store state using the state schema
    pub fn store_state<S: BorshSerialize + StateKey>(&mut self, state: &S) -> Result<(), StateError> {
        let key = state.key();
        let bytes = state.try_to_vec()?;
        self.store_by_key(&key, bytes)
    }

    /// Get state using the state schema
    pub fn get_state<S: BorshDeserialize + StateKey + Default>(&self) -> Result<Option<S>, StateError> {
        let key = S::default().key();
        self.get(&key)
    }

    /// Delete state using the state schema
    pub fn delete_state<S: BorshDeserialize + StateKey + Default>(&mut self) -> Result<Option<S>, StateError> {
        let key = S::default().key();
        if let Some(state) = self.get_state::<S>()? {
            self.store_by_key(&key, Vec::new())?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// Get all events emitted in this context
    pub fn get_events(&self) -> Vec<Event> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.host.get_events()
        }
        #[cfg(target_arch = "wasm32")]
        {
            Vec::new()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Context {
    pub fn get_balance(&self, account: &WasmlAddress) -> u64 {
        self.host.get_balance(account)
    }

    pub fn set_balance(&mut self, account: &WasmlAddress, amount: u64) {
        self.host.set_balance(account, amount)
    }
}

#[cfg(target_arch = "wasm32")]
impl Context {
    pub fn get_state<S: BorshDeserialize + StateKey + Default>(&self) -> Result<Option<S>, StateError> {
        let key = S::default().key();
        // Call host function to get state
        extern "C" {
            fn get_state(key_ptr: *const u8, key_len: usize) -> i32;
        }
        unsafe {
            let result = get_state(key.as_ptr(), key.len());
            if result < 0 {
                return Err(StateError::StorageFailed);
            }
            if result == 0 {
                return Ok(None);
            }
            let mut value = Vec::with_capacity(result as usize);
            extern "C" {
                fn get_value(value_ptr: *mut u8, value_len: usize) -> i32;
            }
            let result = get_value(value.as_mut_ptr(), result as usize);
            if result < 0 {
                return Err(StateError::StorageFailed);
            }
            value.set_len(result as usize);
            Ok(Some(S::try_from_slice(&value)?))
        }
    }

    pub fn store_state<S: BorshSerialize + StateKey>(&mut self, state: &S) -> Result<(), StateError> {
        let key = state.key();
        let bytes = state.try_to_vec()?;
        // Call host function to store state
        extern "C" {
            fn store_state(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
        }
        unsafe {
            let result = store_state(
                key.as_ptr(),
                key.len(),
                bytes.as_ptr(),
                bytes.len()
            );
            if result == 0 {
                Ok(())
            } else {
                Err(StateError::StorageFailed)
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
