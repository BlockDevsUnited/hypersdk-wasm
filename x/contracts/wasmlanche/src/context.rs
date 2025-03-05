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
    future::{AsyncResult, ContractCallResult},
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_host<H: Host + 'static>(host: H) -> Self {
        Self {
            actor: WasmlAddress::default(), // Default actor address
            host: Box::new(host),
            state: RwLock::new(HostState::default()),
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
        }).map_err(|_| Error::State(String::from("Failed to store state")))?;
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
                Err(Error::State(String::from("Failed to store state")))
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
                return Err(Error::State(String::from("Failed to get state")));
            }
            if result == 0 {
                return Ok(None);
            }
            let mut value = Vec::with_capacity(result as usize);
            let result = get_value(value.as_mut_ptr(), result as usize);
            if result < 0 {
                return Err(Error::State(String::from("Failed to get value")));
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
                Err(Error::State(String::from("Failed to store state")))
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
                return Err(Error::State(String::from("Failed to get state")));
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
                return Err(Error::State(String::from("Failed to get value")));
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
                        return Err(Error::State(String::from("Failed to delete state")));
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
    pub fn get_async<T: BorshDeserialize>(&self, key: &[u8]) -> AsyncResult<Option<T>> {
        // The behavior depends on compile target
        #[cfg(not(target_arch = "wasm32"))]
        {
            // For non-wasm targets, we already have the data in memory
            // Just return a completed future
            if let Ok(Some(bytes)) = self.get_by_key(key) {
                match T::try_from_slice(&bytes) {
                    Ok(value) => AsyncResult::with_result(Ok(Some(value))),
                    Err(e) => AsyncResult::with_result(Err(Error::State(format!("Failed to deserialize value: {}", e)))),
                }
            } else {
                // No data found
                AsyncResult::with_result(Ok(None))
            }
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            // Generate a unique operation ID for this async operation
            match self.generate_operation_id() {
                Ok(_op_id) => {
                    // Store the operation ID in the host state
                    if let Ok(Some(bytes)) = self.get_by_key(key) {
                        // If data is already available, return a completed future with the result
                        match T::try_from_slice(&bytes) {
                            Ok(value) => AsyncResult::with_result(Ok(Some(value))),
                            Err(e) => AsyncResult::with_result(Err(Error::State(format!("Failed to deserialize value: {}", e)))),
                        }
                    } else {
                        // Otherwise, set up the async operation
                        let mut result = AsyncResult::new();
                        
                        // In a real implementation, we would initiate the async operation here
                        // For now, we're just creating a pending future
                        result.resolve(Ok(None));
                        
                        result
                    }
                }
                Err(e) => AsyncResult::with_result(Err(e)),
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_async_wasm<T: BorshDeserialize>(&mut self, key: &[u8]) -> AsyncResult<Option<T>> {
        // Generate a unique operation ID
        match self.generate_operation_id() {
            Ok(op_id) => {
                // For wasm32 target, we need to call the host function
                extern "C" {
                    fn get_async(key_ptr: *const u8, key_len: usize) -> i32;
                }
                
                let result = unsafe {
                    get_async(key.as_ptr(), key.len())
                };
                
                if result < 0 {
                    AsyncResult::with_result(Err(Error::State(String::from("Failed to start async get operation"))))
                } else {
                    // Create a pending future - the real result will be retrieved later
                    AsyncResult::new()
                }
            },
            Err(e) => AsyncResult::with_result(Err(e)),
        }
    }

    /// Get the result of an async operation
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_async_result<T: BorshDeserialize>(&self, op_id: &str) -> Result<Option<T>, Error> {
        if !self.check_async_operation(op_id) {
            // Operation not completed yet
            return Err(Error::State(String::from("Operation not completed yet")));
        }
        
        // In a real implementation, we would retrieve the result from a result store
        // For simplicity, let's assume we have the result from normal state access
        
        // Example result fetching
        let result_key = [op_id.as_bytes(), b"_result"].concat();
        match self.get_by_key(&result_key) {
            Ok(Some(bytes)) => {
                match T::try_from_slice(&bytes) {
                    Ok(value) => Ok(Some(value)),
                    Err(e) => Err(Error::State(format!("Failed to deserialize async result: {}", e))),
                }
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_async_result<T: BorshDeserialize>(&self, op_id: &str) -> Result<Option<T>, Error> {
        if !self.check_async_operation(op_id) {
            // Operation not completed yet
            return Err(Error::State(String::from("Operation not completed yet")));
        }
        
        // For wasm32 target, we need to call the host function
        extern "C" {
            fn get_async_result(op_id_ptr: *const u8, op_id_len: usize) -> i32;
        }
        
        let result = unsafe {
            get_async_result(op_id.as_ptr(), op_id.len())
        };
        
        if result < 0 {
            Err(Error::State(String::from("Failed to get async result")))
        } else {
            // The result will be available through a special access mechanism
            // For now, this is a placeholder
            Ok(None)
        }
    }

    /// Store a value asynchronously in state
    #[cfg(not(target_arch = "wasm32"))]
    pub fn put_async<T: BorshSerialize>(&mut self, key: &[u8], value: &T) -> Result<String, Error> {
        match value.try_to_vec() {
            Ok(bytes) => self.store_by_key_async(key, bytes),
            Err(e) => Err(Error::State(format!("Failed to serialize value: {}", e))),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn put_async<T: BorshSerialize>(&mut self, key: &[u8], value: &T) -> Result<String, Error> {
        match value.try_to_vec() {
            Ok(bytes) => self.store_by_key_async(key, bytes),
            Err(e) => Err(Error::State(format!("Failed to serialize value: {}", e))),
        }
    }

    /// Store raw bytes asynchronously in state
    #[cfg(not(target_arch = "wasm32"))]
    pub fn store_by_key_async(&mut self, key: &[u8], value: Vec<u8>) -> Result<String, Error> {
        // Generate a unique operation ID
        match self.generate_operation_id() {
            Ok(op_id) => {
                // For now, we directly store it synchronously
                match self.store_by_key(key, value) {
                    Ok(_) => Ok(op_id),
                    Err(e) => Err(e),
                }
            },
            Err(e) => Err(e),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn store_by_key_async(&mut self, key: &[u8], value: Vec<u8>) -> Result<String, Error> {
        // Generate a unique operation ID
        match self.generate_operation_id() {
            Ok(op_id) => {
                // For wasm32 target, call the host function
                extern "C" {
                    fn store_async(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
                }
                
                let result = unsafe {
                    store_async(key.as_ptr(), key.len(), value.as_ptr(), value.len())
                };
                
                if result < 0 {
                    Err(Error::State(String::from("Failed to start async store operation")))
                } else {
                    Ok(op_id)
                }
            },
            Err(e) => Err(e),
        }
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

    /// Call another contract asynchronously
    /// 
    /// This function initiates an asynchronous call to another contract and returns a future
    /// that resolves when the call completes or times out.
    /// 
    /// # Arguments
    /// 
    /// * `target` - The address of the contract to call
    /// * `method` - The name of the method to call
    /// * `args` - The serialized arguments to pass to the method
    /// * `timeout_ms` - Optional timeout in milliseconds, defaults to 10000 (10 seconds)
    /// 
    /// # Returns
    /// 
    /// A future that resolves to the result of the contract call, or an error
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn call_contract(&mut self, target: &WasmlAddress, method: &str, args: &[u8], timeout_ms: Option<u64>) -> Result<Vec<u8>, Error> {
        // Default timeout of 10 seconds if not specified
        let _timeout = timeout_ms.unwrap_or(10000);
        
        // Account for gas cost of cross-contract call
        // A real implementation would calculate this based on the target contract and method
        let estimated_gas = args.len() as u64 + method.len() as u64 + 1000;
        if let Err(e) = self.host.consume_gas(estimated_gas) {
            return Err(Error::Gas(format!("Insufficient gas for cross-contract call: {}", e)));
        }
        
        // In non-WASM mode, we can directly execute the target contract in the simulator
        // In the future, this would involve more complex contract loading and execution
        
        // Create a ContractCallResult to return
        let mut result = ContractCallResult::new();
        
        // For demonstration, we're simulating the async nature:
        // 1. Check if the target contract exists (we'll just check the address is valid)
        if target.as_bytes().len() != 32 {
            result.resolve(Err(format!("Invalid target contract address")));
            return Err(Error::Contract(String::from("Invalid target contract address")));
        }
        
        // 2. Set up the result - this would actually call the contract in a real implementation
        let response = self.host.call_contract(target, method, args)?;
        
        // Return the response
        Ok(response)
    }
    
    /// Call another contract asynchronously in WASM environment
    /// 
    /// This function initiates an asynchronous call to another contract and returns a future
    /// that resolves when the call completes or times out.
    #[cfg(target_arch = "wasm32")]
    pub async fn call_contract(&mut self, target: &WasmlAddress, method: &str, args: &[u8], timeout_ms: Option<u64>) -> Result<Vec<u8>, Error> {
        // Default timeout of 10 seconds if not specified
        let timeout = timeout_ms.unwrap_or(10000);
        
        // Generate a unique operation ID for this async operation
        let op_id = self.generate_operation_id()?;
        
        extern "C" {
            fn call_contract(
                target_ptr: *const u8, 
                target_len: usize, 
                method_ptr: *const u8, 
                method_len: usize, 
                args_ptr: *const u8, 
                args_len: usize,
                timeout_ms: u64,
                op_id_ptr: *const u8,
                op_id_len: usize
            ) -> i32;
        }
        
        let target_bytes = target.as_bytes();
        let method_bytes = method.as_bytes();
        let op_id_bytes = op_id.as_bytes();
        
        // Initiate the cross-contract call
        let result = unsafe {
            call_contract(
                target_bytes.as_ptr(),
                target_bytes.len(),
                method_bytes.as_ptr(),
                method_bytes.len(),
                args.as_ptr(),
                args.len(),
                timeout,
                op_id_bytes.as_ptr(),
                op_id_bytes.len()
            )
        };
        
        if result < 0 {
            return Err(Error::Contract(String::from("Failed to initiate cross-contract call")));
        }
        
        // Now we wait for the operation to complete
        // Create a ContractCallResult
        let mut call_result = ContractCallResult::new();
        
        // For now, we'll just set it to pending
        // The actual result will be fetched and resolved by the host environment
        
        // In a real implementation, we would check for timeout and completion
        // For now, we return an empty successful result
        call_result.resolve(Ok(Vec::new()));
        
        // Convert ContractCallResult to Result<Vec<u8>, Error>
        match call_result.await {
            Ok(data) => Ok(data),
            Err(e) => Err(Error::Contract(e)),
        }
    }

    /// Get the current timestamp (milliseconds since epoch)
    /// 
    /// This method provides access to the blockchain's current timestamp,
    /// allowing for time-based contract logic.
    #[cfg(all(not(target_arch = "wasm32"), feature = "std"))]
    pub async fn current_timestamp(&self) -> Result<u64, Error> {
        // In the simulator, we use the local system time
        // In a real blockchain implementation, this would be the block timestamp
        use std::time::{SystemTime, UNIX_EPOCH};
        
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => Ok(duration.as_millis() as u64),
            Err(_) => Err(Error::Unknown(String::from("Failed to get current timestamp"))),
        }
    }
    
    /// Get the current timestamp (milliseconds since epoch)
    /// This is a non-std implementation for testing in no_std environments
    #[cfg(all(not(target_arch = "wasm32"), not(feature = "std")))]
    pub async fn current_timestamp(&self) -> Result<u64, Error> {
        // For no_std environments, return a mock timestamp
        Ok(42_000_000_000) // Mock timestamp
    }
    
    /// Get the current timestamp (milliseconds since epoch)
    /// 
    /// This method provides access to the blockchain's current timestamp,
    /// allowing for time-based contract logic.
    #[cfg(target_arch = "wasm32")]
    pub async fn current_timestamp(&self) -> Result<u64, Error> {
        extern "C" {
            fn get_timestamp() -> u64;
        }
        
        // Call the host function to get the current timestamp
        let timestamp = unsafe { get_timestamp() };
        Ok(timestamp)
    }

    /// Get the current timestamp (milliseconds since epoch)
    /// 
    /// This method provides access to the blockchain's current timestamp,
    /// allowing for time-based contract logic.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_timestamp(&self) -> u64 {
        // In a real implementation, this would be provided by the host environment
        // For testing, we can use the system time
        #[cfg(feature = "std")]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
        }
        
        #[cfg(not(feature = "std"))]
        {
            // For no_std environments, return a mock timestamp
            // In a real implementation, this would come from the host
            42_000_000_000 // Mock timestamp
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

#[cfg(target_arch = "wasm32")]
impl Context {
    /// Get the value attached to the current contract call.
    pub fn value(&self) -> u64 {
        extern "C" {
            fn get_call_value() -> u64;
        }
        unsafe {
            get_call_value()
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imports {
    extern "C" {
        pub fn set_call_result(ptr: *const u8, len: usize);
    }
}
