#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, vec::Vec, vec, format};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use borsh::{BorshDeserialize, BorshSerialize};
use spin::RwLock;

use crate::{
    error::Error,
    events::Event,
    host::{Host, HostImpl, HostState},
    state::StateKey,
    types::WasmlAddress,
};

// Define StateError as a type alias for Error
pub type StateError = Error;

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
        }).map_err(|_| Error::State("Failed to store state"))?;
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
                Err(Error::State("Failed to store state"))
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
            fn get_value(value_ptr: *mut u8, capacity: usize) -> i32;
        }
        unsafe {
            let result = get_state(key.as_ptr(), key.len());
            if result < 0 {
                return Err(Error::State("Failed to get state"));
            }
            if result == 0 {
                return Ok(None);
            }
            let mut value = Vec::with_capacity(result as usize);
            let result = get_value(value.as_mut_ptr(), result as usize);
            if result < 0 {
                return Err(Error::State("Failed to get value"));
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
        let mut value = Vec::new();
        state.serialize(&mut value)?;
        extern "C" {
            fn store_state(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
        }
        unsafe {
            let result = store_state(key.as_ptr(), key.len(), value.as_ptr(), value.len());
            if result == 0 {
                Ok(())
            } else {
                Err(Error::State("Failed to store state"))
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
    pub fn get_state<S: BorshDeserialize + StateKey>(&self) -> Result<Option<S>, StateError> {
        let key = S::key_static();
        extern "C" {
            fn get_state(key_ptr: *const u8, key_len: usize) -> i32;
            fn get_value(value_ptr: *mut u8, capacity: usize) -> i32;
        }
        unsafe {
            let result = get_state(key.as_ptr(), key.len());
            if result < 0 {
                return Err(Error::State("Failed to get state"));
            }
            if result == 0 {
                return Ok(None);
            }
            let mut value = Vec::with_capacity(result as usize);
            if result > 0 {
                value.set_len(result as usize);
            }
            let result = get_value(value.as_mut_ptr(), result as usize);
            if result < 0 {
                return Err(Error::State("Failed to get value"));
            }
            value.set_len(result as usize);
            Ok(Some(S::try_from_slice(&value)?))
        }
    }

    /// Delete state using the state schema
    pub fn delete_state<S: BorshDeserialize + StateKey>(&mut self) -> Result<Option<S>, StateError> {
        match self.get_state() {
            Ok(Some(state)) => {
                let key = S::key_static();
                extern "C" {
                    fn delete_state(key_ptr: *const u8, key_len: usize) -> i32;
                }
                unsafe {
                    let result = delete_state(key.as_ptr(), key.len());
                    if result < 0 {
                        return Err(Error::State("Failed to delete state"));
                    }
                }
                Ok(Some(state))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
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

    /// Asynchronously get a value from contract state storage
    /// Returns an operation ID that can be used to check completion status
    #[cfg(target_arch = "wasm32")]
    pub fn get_async<T: BorshDeserialize>(&self, _key: &[u8]) -> Result<String, StateError> {
        // In WebAssembly, generate an operation ID
        let id_bytes = unsafe {
            extern "C" {
                fn generate_operation_id() -> i32;
            }
            let raw_id = generate_operation_id();
            Vec::from(crate::memory::read_memory(raw_id))
        };

        String::from_utf8(id_bytes).map_err(|_| Error::Serialization("Failed to convert operation ID to string"))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_async<T: BorshDeserialize>(&mut self, _key: &[u8]) -> Result<String, StateError> {
        // Mock implementation for non-WASM environments
        Ok(String::from("test-op-id-12345"))
    }

    /// Check if an async operation has completed
    pub fn check_async_operation(&self, _op_id: &str) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Mock implementation for testing
            true
        }
        #[cfg(target_arch = "wasm32")]
        {
            extern "C" {
                fn check_async_operation(op_id_ptr: *const u8, op_id_len: usize) -> i32;
            }
            
            let op_id_bytes = _op_id.as_bytes();
            
            unsafe {
                check_async_operation(op_id_bytes.as_ptr(), op_id_bytes.len()) > 0
            }
        }
    }

    /// Get the result of a completed async operation
    #[cfg(target_arch = "wasm32")]
    pub fn get_async_result<T: BorshDeserialize>(&self, op_id: &str) -> Result<Option<T>, StateError> {
        if !self.check_async_operation(op_id) {
            return Ok(None);
        }

        // Get the operation result
        let result_bytes = unsafe {
            extern "C" {
                fn get_async_result(op_id_ptr: *const u8, op_id_len: usize) -> i32;
            }
            let op_id_bytes = op_id.as_bytes();
            let result_id = get_async_result(op_id_bytes.as_ptr(), op_id_bytes.len());
            if result_id <= 0 {
                return Ok(None);
            }
            Vec::from(crate::memory::read_memory(result_id))
        };

        match result_bytes.len() {
            0 => Ok(None),
            _ => match T::try_from_slice(&result_bytes) {
                Ok(value) => Ok(Some(value)),
                Err(_) => Err(Error::Serialization("Failed to deserialize operation result")),
            },
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_async_result<T: BorshDeserialize>(&self, _op_id: &str) -> Result<Option<T>, StateError> {
        // Mock implementation for testing
        // In a real implementation, this would check if the operation has completed
        // and return the result if available
        Ok(None)
    }

    /// Store a value asynchronously in state
    #[cfg(target_arch = "wasm32")]
    pub fn put_async<T: BorshSerialize>(&mut self, key: &[u8], value: &T) -> Result<String, StateError> {
        let mut bytes = Vec::new();
        value.serialize(&mut bytes).map_err(|_| Error::Serialization("Failed to serialize value"))?;
        self.store_by_key_async(key, bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn put_async<T: BorshSerialize>(&mut self, _key: &[u8], _value: &T) -> Result<String, StateError> {
        // Mock implementation for testing
        Ok(String::from("test-op-id-12345"))
    }

    /// Store raw bytes asynchronously in state
    pub fn store_by_key_async(&mut self, _key: &[u8], _value: Vec<u8>) -> Result<String, StateError> {
        // Generate a unique operation ID
        let op_id = self.generate_operation_id()?;
        
        // For now, we just perform the synchronous operation
        // In the future, this would queue the operation and return immediately
        #[cfg(target_arch = "wasm32")]
        {
            extern "C" {
                fn store_async(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
            }
            
            unsafe {
                let result = store_async(_key.as_ptr(), _key.len(), _value.as_ptr(), _value.len());
                if result < 0 {
                    return Err(Error::State("Failed to start async store operation"));
                }
            }
        }
        
        Ok(op_id)
    }
    
    /// Generate a unique operation ID for async operations
    #[cfg(not(target_arch = "wasm32"))]
    pub fn generate_operation_id(&mut self) -> Result<String, StateError> {
        // This is a simple mock implementation
        // Just return a fixed string for testing
        Ok(String::from("test-op-id-12345"))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn generate_operation_id(&self) -> Result<String, StateError> {
        // This is a simplified implementation for wasm environment
        // Generate a simple deterministic operation ID
        extern "C" {
            fn random_bytes(ptr: *mut u8, len: usize) -> u32;
        }
        
        // Use 16 bytes for the operation ID
        let mut id_bytes = vec![0u8; 16];
        
        // Get random bytes to use as the operation ID
        // In a real implementation, this would be more sophisticated
        unsafe {
            random_bytes(id_bytes.as_mut_ptr(), id_bytes.len());
        }
        
        // Convert to hex string
        let id = id_bytes.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
            
        Ok(id)
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
