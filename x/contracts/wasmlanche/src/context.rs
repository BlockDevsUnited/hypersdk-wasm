#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, vec::Vec};

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
#[derive(BorshSerialize)]
pub struct Context {
    pub actor: WasmlAddress,
    #[borsh_skip]
    host: Box<dyn Host>,
    #[borsh_skip]
    state: RwLock<HostState>,
}

#[cfg(target_arch = "wasm32")]
#[derive(BorshSerialize)]
pub struct Context {
    pub actor: WasmlAddress,
    #[borsh_skip]
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
    #[cfg(not(target_arch = "wasm32"))]
    pub fn store_by_key(&mut self, key: &[u8], value: Vec<u8>) -> Result<(), StateError> {
        let mut state = self.state.write();
        state.store(key, value.clone());
        self.host.emit_event(Event::StateChange {
            key: key.to_vec(),
            value,
        }).map_err(|_| StateError::StorageFailed)?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn store_by_key(&mut self, key: &[u8], value: Vec<u8>) -> Result<(), StateError> {
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

    /// Get a value from state
    pub fn get<T: BorshDeserialize>(&self, key: &[u8]) -> Result<Option<T>, StateError> {
        let bytes = self.get_by_key(key)?;
        match bytes {
            Some(bytes) => Ok(Some(T::try_from_slice(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Get raw bytes from state
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_by_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StateError> {
        let state = self.state.read();
        Ok(state.get(key).cloned())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_by_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StateError> {
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

    /// Store state using the state schema
    #[cfg(not(target_arch = "wasm32"))]
    pub fn store_state<S: BorshSerialize + StateKey>(&mut self, state: &S) -> Result<(), StateError> {
        let key = state.key();
        let bytes = state.try_to_vec()?;
        self.store_by_key(&key, bytes)
    }

    #[cfg(target_arch = "wasm32")]
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

    /// Get state using the state schema
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_state<S: BorshDeserialize + StateKey + Default>(&self) -> Result<Option<S>, StateError> {
        let key = S::default().key();
        self.get(&key)
    }

    #[cfg(target_arch = "wasm32")]
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

    /// Delete state using the state schema
    pub fn delete_state<S: BorshDeserialize + StateKey + Default>(&mut self) -> Result<Option<S>, StateError> {
        let key = S::default().key();
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(state) = self.get_state::<S>()? {
                self.store_by_key(&key, Vec::new())?;
                Ok(Some(state))
            } else {
                Ok(None)
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(state) = self.get_state::<S>()? {
                // Call host function to delete state
                extern "C" {
                    fn delete_state(key_ptr: *const u8, key_len: usize) -> i32;
                }
                unsafe {
                    let result = delete_state(key.as_ptr(), key.len());
                    if result < 0 {
                        return Err(StateError::StorageFailed);
                    }
                }
                Ok(Some(state))
            } else {
                Ok(None)
            }
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

    /// Get a value asynchronously from state
    #[cfg(target_arch = "wasm32")]
    pub fn get_async(&self, key: &[u8]) -> Result<String, StateError> {
        extern "C" {
            fn get_async(
                key_ptr: *const u8,
                key_len: usize,
            ) -> u64;
        }

        let id_bytes = unsafe {
            let raw_id = get_async(key.as_ptr(), key.len());
            Vec::from(crate::memory::read_memory(raw_id))
        };

        String::from_utf8(id_bytes).map_err(|_| StateError::InvalidArgument)
    }

    /// Check if an async operation has completed
    #[cfg(target_arch = "wasm32")]
    pub fn check_async_operation(&self, op_id: &str) -> bool {
        extern "C" {
            fn check_async(
                id_ptr: *const u8,
                id_len: usize,
            ) -> u32;
        }

        let id_bytes = op_id.as_bytes();
        unsafe {
            check_async(id_bytes.as_ptr(), id_bytes.len()) != 0
        }
    }

    /// Get the result of a completed async operation
    #[cfg(target_arch = "wasm32")]
    pub fn get_async_result<T: BorshDeserialize>(&self, op_id: &str) -> Result<Option<T>, StateError> {
        extern "C" {
            fn get_async_result(
                id_ptr: *const u8,
                id_len: usize,
            ) -> u64;
        }

        let id_bytes = op_id.as_bytes();
        let result_bytes = unsafe {
            let raw_result = get_async_result(id_bytes.as_ptr(), id_bytes.len());
            Vec::from(crate::memory::read_memory(raw_result))
        };

        match result_bytes.len() {
            0 => Ok(None),
            _ => match T::try_from_slice(&result_bytes) {
                Ok(value) => Ok(Some(value)),
                Err(_) => Err(StateError::InvalidData),
            },
        }
    }

    /// Store a value asynchronously in state
    #[cfg(target_arch = "wasm32")]
    pub fn put_async<T: BorshSerialize>(&mut self, key: &[u8], value: &T) -> Result<String, StateError> {
        let mut bytes = Vec::new();
        value.serialize(&mut bytes).map_err(|_| StateError::InvalidData)?;
        self.store_by_key_async(key, bytes)
    }

    /// Store raw bytes asynchronously in state
    #[cfg(target_arch = "wasm32")]
    pub fn store_by_key_async(&mut self, key: &[u8], value: Vec<u8>) -> Result<String, StateError> {
        extern "C" {
            fn put_async(
                key_ptr: *const u8,
                key_len: usize,
                val_ptr: *const u8,
                val_len: usize,
            ) -> u64;
        }

        let id_bytes = unsafe {
            let raw_id = put_async(
                key.as_ptr(),
                key.len(),
                value.as_ptr(),
                value.len(),
            );
            Vec::from(crate::memory::read_memory(raw_id))
        };

        String::from_utf8(id_bytes).map_err(|_| StateError::InvalidArgument)
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
