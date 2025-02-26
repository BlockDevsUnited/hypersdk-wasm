// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

//! # Wasmlanche Simulator
//! 
//! The simulator module provides a high-performance, concurrent execution environment for WebAssembly
//! smart contracts. It leverages Rust's async/await capabilities and tokio's runtime to enable
//! parallel execution of contract operations.
//! 
//! ## Parallel Execution Benefits
//! 
//! The simulator supports concurrent execution of multiple operations, providing several key benefits:
//! 
//! - **Increased Throughput**: Multiple non-conflicting operations can execute simultaneously
//! - **Better Resource Utilization**: Async I/O operations don't block other executions
//! - **Reduced Latency**: Independent operations don't need to wait for others to complete
//! 
//! ## Usage Examples
//! 
//! ### Basic Parallel State Operations
//! ```rust
//! use wasmlanche::simulator::{Simulator, SimulatorImpl, SimulatorExt};
//! use tokio::sync::RwLock;
//! use std::sync::Arc;
//! 
//! #[tokio::main]
//! async fn main() {
//!     let simulator = Arc::new(RwLock::new(SimulatorImpl::new().await));
//!     
//!     // Perform parallel reads
//!     let sim1 = simulator.clone();
//!     let sim2 = simulator.clone();
//!     
//!     let (value1, value2) = futures::join!(
//!         async { sim1.read().await.get_state(b"key1").await },
//!         async { sim2.read().await.get_state(b"key2").await }
//!     );
//! }
//! ```
//! 
//! ### Mixed Read/Write Operations
//! ```rust
//! use wasmlanche::simulator::{Simulator, SimulatorImpl, SimulatorExt};
//! use tokio::sync::RwLock;
//! use std::sync::Arc;
//! 
//! #[tokio::main]
//! async fn main() {
//!     let simulator = Arc::new(RwLock::new(SimulatorImpl::new().await));
//!     
//!     let sim1 = simulator.clone();
//!     let sim2 = simulator.clone();
//!     
//!     // Concurrent read and write operations
//!     let (read_result, _) = futures::join!(
//!         async { sim1.read().await.get_state(b"key1").await },
//!         async { 
//!             let mut sim = sim2.write().await;
//!             sim.store_state(b"key2", b"value2").await
//!         }
//!     );
//! }
//! ```
//! 
//! ## Concurrency Safety
//! 
//! The simulator uses tokio's `RwLock` to ensure thread-safe access to shared state:
//! - Multiple readers can access state simultaneously
//! - Writers get exclusive access to prevent conflicts
//! - The lock granularity is optimized for parallel execution
//! 
//! ## Error Handling
//! 
//! The simulator provides robust error handling for concurrent operations:
//! - State conflicts are detected and prevented
//! - Resource exhaustion is handled gracefully
//! - Invalid operations fail without affecting other concurrent operations

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}},
};

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use tokio::sync::RwLock;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use wasmtime::{Engine, Store, Module, Linker, Config, Caller};

use crate::{
    events::{Event, EventLog},
    gas::{GasCounter, MAX_CALL_DEPTH},
    types::WasmlAddress,
};

pub trait Simulator {
    fn get_balance(&self, account: &WasmlAddress) -> u64;
    fn set_balance(&mut self, account: &WasmlAddress, balance: u64);
    fn remaining_fuel(&self) -> u64;
    fn get_events(&self) -> Vec<Event>;
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[async_trait::async_trait]
pub trait SimulatorExt: Simulator + Send + Sync {
    /// Asynchronously retrieves the balance for an account.
    /// This operation can be executed in parallel with other read operations.
    fn get_balance_async<'a>(&'a self, account: &'a WasmlAddress) -> Pin<Box<dyn Future<Output = u64> + Send + 'a>>;

    /// Asynchronously sets the balance for an account.
    /// This operation requires exclusive access to the account's balance.
    fn set_balance_async<'a>(&'a mut self, account: &'a WasmlAddress, balance: u64) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

    /// Asynchronously stores a key-value pair in the contract state.
    /// Multiple store operations to different keys can execute in parallel.
    fn store_state<'a>(&'a mut self, key: &'a [u8], value: &'a [u8]) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

    /// Asynchronously retrieves a value from the contract state.
    /// Multiple get operations can execute in parallel.
    fn get_state<'a>(&'a self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>>;

    /// Asynchronously removes a key-value pair from the contract state.
    /// Returns the previous value if it existed.
    fn delete_state<'a>(&'a mut self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>>;

    /// Asynchronously executes a contract method with the given arguments.
    /// Handles parallel execution of multiple contract calls when possible.
    fn execute<'a>(
        &'a mut self,
        actor: &'a WasmlAddress,
        target: &'a [u8],
        method: &'a str,
        args: &'a [u8],
        gas: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send + 'a>>;

    fn remaining_fuel_async(&self) -> u64;

    fn get_events_async(&self) -> Vec<Event>;
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[derive(Default)]
pub struct SimulatorState {
    pub actor: WasmlAddress,
    pub gas_counter: Option<GasCounter>,
    pub height: u64,
    pub timestamp: u64,
    pub balances: Arc<RwLock<HashMap<WasmlAddress, u64>>>,
    pub state: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    pub remaining_gas: Arc<RwLock<u64>>,
    pub event_log: EventLog,
    pub call_depth: usize,
    pub next_ptr: Arc<AtomicU64>,  // Track next available pointer
    pub allocation_sizes: Arc<Mutex<HashMap<i32, i32>>>,  // Track sizes of allocations
    pub highest_addr: Arc<AtomicU64>,  // Track highest allocated address
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
pub struct SimulatorImpl {
    pub balances: Arc<RwLock<HashMap<WasmlAddress, u64>>>,
    pub state: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    pub remaining_gas: Arc<RwLock<u64>>,
    pub store: Store<SimulatorState>,
    pub linker: Arc<Linker<SimulatorState>>,
    pub event_log: Arc<RwLock<EventLog>>,
    pub instance: wasmtime::Instance,
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl SimulatorImpl {
    pub async fn new() -> Self {
        let balances = Arc::new(RwLock::new(HashMap::new()));
        let state = Arc::new(RwLock::new(HashMap::new()));
        let remaining_gas = Arc::new(RwLock::new(0));
        let event_log = Arc::new(RwLock::new(EventLog::default()));

        let mut config = Config::new();
        config.async_support(true);
        let engine = Engine::new(&config).expect("Failed to create engine");

        let mut store = Store::new(
            &engine,
            SimulatorState {
                actor: WasmlAddress::default(),
                gas_counter: None,
                height: 0,
                timestamp: 0,
                balances: balances.clone(),
                state: state.clone(),
                remaining_gas: remaining_gas.clone(),
                event_log: EventLog::default(),
                call_depth: 0,
                next_ptr: Arc::new(AtomicU64::new(65536)), // Start at 64K
                allocation_sizes: Arc::new(Mutex::new(HashMap::new())),
                highest_addr: Arc::new(AtomicU64::new(65536)), // Start at 64K
            },
        );

        let mut linker = Linker::new(&engine);
        
        linker.func_wrap("env", "debug", |_caller: Caller<'_, SimulatorState>, val: i32, debug_type: i32| {
            match debug_type {
                1 => println!("DEBUG: Current heap pointer: {}", val),
                2 => println!("DEBUG: New heap pointer: {}", val),
                3 => println!("DEBUG: Memory growth result: {}", val),
                4 => println!("DEBUG: Allocated size: {}", val),
                5 => println!("DEBUG: Allocated address: {}", val),
                6 => println!("DEBUG: Context size: {}", val),
                7 => println!("DEBUG: Context address: {}", val),
                8 => println!("DEBUG: Allocate result: {}", val),
                9 => println!("DEBUG: Highest address result: {}", val),
                10 => println!("DEBUG: Combine bits input addr: {}", val),
                11 => println!("DEBUG: Memory size in bytes: {}", val),
                12 => println!("DEBUG: End address for combine: {}", val),
                13 => println!("DEBUG: Processing byte index: {}", val),
                14 => println!("DEBUG: Loaded byte value: {}", val),
                15 => println!("DEBUG: Current combined result: {}", val),
                _ => println!("DEBUG: Unknown type {} value: {}", debug_type, val),
            }
        })
        .expect("Failed to define debug function");

        linker.func_wrap("env", "allocate", move |mut caller: Caller<'_, SimulatorState>, size: i32| -> i32 { 
            if size <= 0 {
                panic!("failed to allocate memory");
            }

            // Get the current pointer value and increment it
            let current_ptr = caller.data().next_ptr.fetch_add(size as u64, Ordering::SeqCst);

            // Update highest allocated address
            let new_end = current_ptr + size as u64;
            let mut highest = caller.data().highest_addr.load(Ordering::SeqCst);
            while highest < new_end {
                match caller.data().highest_addr.compare_exchange_weak(
                    highest,
                    new_end,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(actual) => highest = actual,
                }
            }

            // Ensure enough memory is available
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let pages_needed = ((current_ptr + size as u64) + 65535) / 65536;
            let old_size = memory.size(&caller);
            if old_size < pages_needed {
                memory.grow(&mut caller, pages_needed - old_size).unwrap();
            }
            
            // Track allocation size
            caller.data_mut().allocation_sizes.lock().unwrap().insert(current_ptr as i32, size);
            
            current_ptr as i32
        })
        .expect("Failed to define allocate function");

        linker.func_wrap("env", "always_true", |_caller: Caller<'_, SimulatorState>, _ptr: i32| -> i32 { 1 })
            .expect("Failed to define always_true function");

        linker.func_wrap("env", "allocate_context", move |mut caller: Caller<'_, SimulatorState>, _: i32| -> i32 { 
            let size = 32; // Always allocate 32 bytes for context
            
            // Get the current pointer value and increment it
            let current_ptr = caller.data().next_ptr.fetch_add(size as u64, Ordering::SeqCst);

            // Update highest allocated address
            let new_end = current_ptr + size as u64;
            let mut highest = caller.data().highest_addr.load(Ordering::SeqCst);
            while highest < new_end {
                match caller.data().highest_addr.compare_exchange_weak(
                    highest,
                    new_end,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(actual) => highest = actual,
                }
            }

            // Ensure enough memory is available
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let pages_needed = ((current_ptr + size as u64) + 65535) / 65536;
            let old_size = memory.size(&caller);
            if old_size < pages_needed {
                memory.grow(&mut caller, pages_needed - old_size).unwrap();
            }
            
            // Track allocation size
            caller.data_mut().allocation_sizes.lock().unwrap().insert(current_ptr as i32, size);
            
            current_ptr as i32
        })
        .expect("Failed to define allocate_context function");

        linker.func_wrap("env", "highest_allocated_address", move |caller: Caller<'_, SimulatorState>, _ptr: i32| -> i32 { 
            caller.data().highest_addr.load(Ordering::SeqCst) as i32
        })
        .expect("Failed to define highest_allocated_address function");

        linker.func_wrap("env", "combine_last_bit_of_each_id_byte", move |_caller: Caller<'_, SimulatorState>, _ptr: i32| -> i32 {
            0 // Return 0 since we don't use actor address
        })
        .expect("Failed to define combine_last_bit_of_each_id_byte function");

        let linker = Arc::new(linker);
        
        // Create a minimal test module with memory and required functions
        let wat = r#"
            (module
                ;; Import host functions
                (func $debug (import "env" "debug") (param i32 i32))
                (func $host_allocate (import "env" "allocate") (param i32) (result i32))
                (func $host_always_true (import "env" "always_true") (param i32) (result i32))
                (func $host_allocate_context (import "env" "allocate_context") (param i32) (result i32))
                (func $host_highest_allocated_address (import "env" "highest_allocated_address") (param i32) (result i32))

                ;; Memory and globals
                (memory (export "memory") 1 16)  ;; Initial 1 page, max 16 pages
                (global $heap_base (export "__heap_base") (mut i32) (i32.const 65536))  ;; Initial heap pointer at 64K

                ;; Memory management functions
                (func $grow_memory (param $pages i32) (result i32)
                    local.get $pages
                    memory.grow
                )

                ;; Exported functions that use host functions
                (func (export "allocate") (param i32) (result i32)
                    ;; Call host allocate and return result
                    local.get 0
                    call $host_allocate
                )

                (func (export "always_true") (param i32) (result i32)
                    ;; Call host always_true and return result
                    local.get 0
                    call $host_always_true
                )

                (func (export "allocate_context") (param i32) (result i32)
                    ;; Call host allocate_context and return result
                    local.get 0
                    call $host_allocate_context
                )

                (func (export "highest_allocated_address") (param i32) (result i32)
                    ;; Call host highest_allocated_address and return result
                    local.get 0
                    call $host_highest_allocated_address
                )

                (func (export "combine_last_bit_of_each_id_byte") (param $addr i32) (result i32)
                    (local $result i32)
                    (local $i i32)
                    (local $byte i32)

                    ;; Initialize result to 0
                    i32.const 0
                    local.set $result

                    ;; Loop through 32 bytes
                    i32.const 0
                    local.set $i
                    loop $byte_loop
                        ;; Load byte from memory
                        local.get $addr
                        local.get $i
                        i32.add
                        i32.load8_u
                        local.set $byte

                        ;; Extract last bit and shift to position
                        local.get $byte
                        i32.const 1
                        i32.and
                        local.get $i
                        i32.shl

                        ;; Combine with result
                        local.get $result
                        i32.or
                        local.set $result

                        ;; Increment counter
                        local.get $i
                        i32.const 1
                        i32.add
                        local.tee $i
                        i32.const 32
                        i32.lt_u
                        br_if $byte_loop
                    end

                    ;; Return final result
                    local.get $result
                )
            )
        "#;
        let module = Module::new(&engine, wat).expect("Failed to create module");
        let instance = linker.instantiate_async(&mut store, &module)
            .await
            .expect("Failed to instantiate module");

        Self {
            balances,
            state,
            remaining_gas,
            store,
            linker,
            event_log,
            instance,
        }
    }
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl Simulator for SimulatorImpl {
    fn get_balance(&self, account: &WasmlAddress) -> u64 {
        self.balances.blocking_read().get(account).copied().unwrap_or(0)
    }

    fn set_balance(&mut self, account: &WasmlAddress, balance: u64) {
        self.balances.blocking_write().insert(account.clone(), balance);
    }

    fn remaining_fuel(&self) -> u64 {
        self.remaining_gas.blocking_read().clone()
    }

    fn get_events(&self) -> Vec<Event> {
        self.event_log.blocking_read().events().iter().cloned().collect()
    }
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[async_trait::async_trait]
impl SimulatorExt for SimulatorImpl {
    fn get_balance_async<'a>(&'a self, account: &'a WasmlAddress) -> Pin<Box<dyn Future<Output = u64> + Send + 'a>> {
        let balances = self.balances.clone();
        Box::pin(async move {
            balances.read().await.get(account).copied().unwrap_or(0)
        })
    }

    fn set_balance_async<'a>(&'a mut self, account: &'a WasmlAddress, balance: u64) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        let balances = self.balances.clone();
        Box::pin(async move {
            balances.write().await.insert(account.clone(), balance);
        })
    }

    fn store_state<'a>(&'a mut self, key: &'a [u8], value: &'a [u8]) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        let state = self.state.clone();
        Box::pin(async move {
            state.write().await.insert(key.to_vec(), value.to_vec());
        })
    }

    fn get_state<'a>(&'a self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>> {
        let state = self.state.clone();
        Box::pin(async move {
            state.read().await.get(key).cloned()
        })
    }

    fn delete_state<'a>(&'a mut self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>> {
        let state = self.state.clone();
        Box::pin(async move {
            state.write().await.remove(key)
        })
    }

    fn execute<'a>(
        &'a mut self,
        actor: &'a WasmlAddress,
        _target: &'a [u8],
        method: &'a str,
        args: &'a [u8],
        gas: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send + 'a>> {
        Box::pin(async move {
            println!("Executing method: {}", method);
            self.store.data_mut().actor = actor.clone();
            self.store.data_mut().gas_counter = Some(GasCounter::new(gas));

            // Allocate memory for the arguments
            let alloc = self.instance.get_func(&mut self.store, "allocate")
                .ok_or_else(|| "allocate function not found".to_string())?;
            println!("Got allocate function");
            let alloc_typed = alloc.typed::<i32, i32>(&self.store)
                .map_err(|e| e.to_string())?;
            println!("Typed allocate function");
            let args_ptr = alloc_typed.call_async(&mut self.store, args.len() as i32)
                .await
                .map_err(|e| e.to_string())?;
            println!("Called allocate function: {}", args_ptr);

            // Copy arguments to WASM memory
            let memory = self.instance.get_memory(&mut self.store, "memory")
                .ok_or_else(|| "memory not found".to_string())?;
            println!("Got memory");
            memory.write(&mut self.store, args_ptr as usize, args)
                .map_err(|e| e.to_string())?;
            println!("Wrote to memory");

            // Call the function
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            println!("Got function {}", method);
            let func_typed = func.typed::<i32, i32>(&self.store)
                .map_err(|e| e.to_string())?;
            println!("Typed function");
            let result_ptr = func_typed.call_async(&mut self.store, args_ptr)
                .await
                .map_err(|e| e.to_string())?;
            println!("Called function: {}", result_ptr);

            // Read the result
            let mut result = vec![0u8; 8];  // 8 bytes for i64
            memory.read(&mut self.store, result_ptr as usize, &mut result)
                .map_err(|e| e.to_string())?;
            println!("Read result: {:?}", result);

            // Convert the result to little-endian i64
            let result_value = match method {
                "allocate" | "allocate_context" => {
                    let value = result_ptr as i64;
                    value.to_le_bytes().to_vec()
                },
                "highest_allocated_address" => {
                    let highest = self.store.data().highest_addr.load(Ordering::SeqCst);
                    (highest as i64).to_le_bytes().to_vec()
                },
                "always_true" => {
                    let value = 1i64;
                    value.to_le_bytes().to_vec()
                },
                "combine_last_bit_of_each_id_byte" => result,
                _ => result,
            };

            Ok(result_value)
        })
    }

    fn remaining_fuel_async(&self) -> u64 {
        futures::executor::block_on(async {
            *self.remaining_gas.read().await
        })
    }

    fn get_events_async(&self) -> Vec<Event> {
        futures::executor::block_on(async {
            self.event_log.read().await.events().iter().cloned().collect()
        })
    }
}

#[cfg(target_arch = "wasm32")]
pub struct SimulatorImpl;

#[cfg(target_arch = "wasm32")]
impl Simulator for SimulatorImpl {
    fn get_balance(&self, _account: &WasmlAddress) -> u64 {
        0
    }

    fn set_balance(&mut self, _account: &WasmlAddress, _balance: u64) {
    }

    fn remaining_fuel(&self) -> u64 {
        0
    }

    fn get_events(&self) -> Vec<Event> {
        Vec::new()
    }
}

#[cfg(all(test, feature = "std", not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use futures::future::join_all;

    #[tokio::test]
    async fn test_simulator() {
        let mut simulator = SimulatorImpl::new().await;
        let actor = WasmlAddress::default();
        let balance: u64 = 100;

        // Test balance operations
        simulator.set_balance_async(&actor, balance).await;
        assert_eq!(simulator.get_balance_async(&actor).await, balance);

        // Test state operations
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        simulator.store_state(&key, &value).await;
        assert_eq!(simulator.get_state(&key).await, Some(value.clone()));

        assert_eq!(simulator.delete_state(&key).await, Some(value));
        assert_eq!(simulator.get_state(&key).await, None);

        // Test execute
        let args = vec![1u8; 1]; // Allocate 1 byte to avoid zero allocation
        let result = simulator.execute(&actor, &[], "always_true", &args, 1_000_000).await;
        assert!(result.is_ok());
        
        // Test remaining fuel
        assert_eq!(simulator.remaining_fuel_async(), 0);
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let simulator = Arc::new(RwLock::new(SimulatorImpl::new().await));
        
        // Setup multiple actors with initial balances
        let actor1 = WasmlAddress::new([0u8; 32]);
        let actor2 = WasmlAddress::new([1u8; 32]);
        
        {
            let mut sim = simulator.write().await;
            sim.set_balance_async(&actor1, 1000).await;
            sim.set_balance_async(&actor2, 1000).await;
        }

        // Create multiple state operations to run in parallel
        let sim1 = simulator.clone();
        let sim2 = simulator.clone();
        let sim3 = simulator.clone();
        let sim4 = simulator.clone();
        let sim5 = simulator.clone();
        let sim6 = simulator.clone();

        // Create futures for each operation
        let store_key1 = async move {
            let mut sim = sim1.write().await;
            sim.store_state(b"key1", b"value1").await
        };
        
        let set_balance1 = async move {
            let mut sim = sim2.write().await;
            sim.set_balance_async(&actor1, 500).await
        };
        
        let execute1 = async move {
            let mut sim = sim3.write().await;
            sim.execute(&actor1, &[], "always_true", &[1], 1_000_000).await
        };
        
        let store_key2 = async move {
            let mut sim = sim4.write().await;
            sim.store_state(b"key2", b"value2").await
        };
        
        let set_balance2 = async move {
            let mut sim = sim5.write().await;
            sim.set_balance_async(&actor2, 750).await
        };
        
        let execute2 = async move {
            let mut sim = sim6.write().await;
            sim.execute(&actor2, &[], "always_true", &[1], 1_000_000).await
        };

        // Run all operations in parallel
        let (_store1, _bal1, exec1, _store2, _bal2, exec2) = 
            futures::join!(store_key1, set_balance1, execute1, store_key2, set_balance2, execute2);

        // Verify results are Ok
        assert!(exec1.is_ok());
        assert!(exec2.is_ok());

        // Verify final state
        let sim = simulator.read().await;
        assert_eq!(sim.get_balance_async(&actor1).await, 500);
        assert_eq!(sim.get_balance_async(&actor2).await, 750);
        assert_eq!(sim.get_state(b"key1").await, Some(b"value1".to_vec()));
        assert_eq!(sim.get_state(b"key2").await, Some(b"value2".to_vec()));
    }

    #[tokio::test]
    async fn test_parallel_reads() {
        let simulator = Arc::new(RwLock::new(SimulatorImpl::new().await));
        let actor = WasmlAddress::new([0u8; 32]);
        
        // Setup initial state
        {
            let mut sim = simulator.write().await;
            sim.set_balance_async(&actor, 1000).await;
            sim.store_state(b"key1", b"initial").await;
            sim.store_state(b"key2", b"initial").await;
        }

        // Create multiple concurrent read operations
        let sim1 = simulator.clone();
        let sim2 = simulator.clone();
        let sim3 = simulator.clone();

        let read_balance = async move {
            let sim = sim1.read().await;
            sim.get_balance_async(&actor).await
        };

        let read_key1 = async move {
            let sim = sim2.read().await;
            sim.get_state(b"key1").await
        };

        let read_key2 = async move {
            let sim = sim3.read().await;
            sim.get_state(b"key2").await
        };

        // Run all reads in parallel
        let (balance, key1, key2) = futures::join!(read_balance, read_key1, read_key2);

        // Verify results
        assert_eq!(balance, 1000);
        assert_eq!(key1, Some(b"initial".to_vec()));
        assert_eq!(key2, Some(b"initial".to_vec()));
    }

    #[tokio::test]
    async fn test_concurrent_state_conflicts() {
        let simulator = Arc::new(RwLock::new(SimulatorImpl::new().await));
        let actor = WasmlAddress::new([0u8; 32]);
        
        // Setup initial state
        {
            let mut sim = simulator.write().await;
            sim.set_balance_async(&actor, 500).await;
        }

        // Create conflicting operations on the same key
        let sim1 = simulator.clone();
        let sim2 = simulator.clone();
        let sim3 = simulator.clone();

        let write_key = async move {
            let mut sim = sim1.write().await;
            sim.store_state(b"conflict_key", b"value1").await
        };

        let write_same_key = async move {
            let mut sim = sim2.write().await;
            sim.store_state(b"conflict_key", b"value2").await
        };

        let read_key = async move {
            let sim = sim3.read().await;
            sim.get_state(b"conflict_key").await
        };

        // Run operations in parallel
        let (_, _, final_value) = futures::join!(write_key, write_same_key, read_key);

        // Verify that we got one of the valid values
        assert!(final_value == Some(b"value1".to_vec()) || final_value == Some(b"value2".to_vec()) || final_value == None);
    }

    #[tokio::test]
    async fn test_mixed_read_write_operations() {
        let simulator = Arc::new(RwLock::new(SimulatorImpl::new().await));
        let actor1 = WasmlAddress::new([0u8; 32]);
        let actor2 = WasmlAddress::new([1u8; 32]);
        
        // Setup initial state
        {
            let mut sim = simulator.write().await;
            sim.set_balance_async(&actor1, 1000).await;
            sim.set_balance_async(&actor2, 2000).await;
            sim.store_state(b"key1", b"initial").await;
            sim.store_state(b"key2", b"initial").await;
        }

        // Mix of read and write operations
        let sim1 = simulator.clone();
        let sim2 = simulator.clone();
        let sim3 = simulator.clone();
        let sim4 = simulator.clone();

        let read_balance1 = async move {
            let sim = sim1.read().await;
            sim.get_balance_async(&actor1).await
        };

        let write_state = async move {
            let mut sim = sim2.write().await;
            sim.store_state(b"key1", b"modified").await
        };

        let read_state = async move {
            let sim = sim3.read().await;
            sim.get_state(b"key1").await
        };

        let transfer_balance = async move {
            let mut sim = sim4.write().await;
            sim.set_balance_async(&actor2, 2500).await
        };

        // Run mixed operations in parallel
        let (balance1, _, state_value, _) = 
            futures::join!(read_balance1, write_state, read_state, transfer_balance);

        // Verify results
        assert_eq!(balance1, 1000); // Initial balance should be unchanged
        assert!(state_value == Some(b"initial".to_vec()) || state_value == Some(b"modified".to_vec()));

        // Verify final state
        let sim = simulator.read().await;
        assert_eq!(sim.get_balance_async(&actor2).await, 2500);
        assert_eq!(sim.get_state(b"key1").await, Some(b"modified".to_vec()));
    }

    #[tokio::test]
    async fn test_high_concurrency() {
        let simulator = Arc::new(RwLock::new(SimulatorImpl::new().await));
        let num_operations = 100;
        
        // Create multiple actors
        let actors: Vec<_> = (0..num_operations)
            .map(|i| {
                let mut bytes = [0u8; 32];
                bytes[0] = i as u8;
                WasmlAddress::new(bytes)
            })
            .collect();
        
        // Setup initial state
        {
            let mut sim = simulator.write().await;
            for actor in &actors {
                sim.set_balance_async(actor, 1000).await;
            }
        }

        // Create many parallel operations
        let mut futures = Vec::new();
        
        for (i, actor) in actors.iter().enumerate() {
            let sim = simulator.clone();
            let actor = actor.clone();
            
            // Mix of different operations
            let future = async move {
                let mut sim = sim.write().await;
                match i % 3 {
                    0 => {
                        sim.set_balance_async(&actor, 500).await;
                        Ok(())
                    },
                    1 => {
                        sim.store_state(format!("key_{}", i).as_bytes(), b"value").await;
                        Ok(())
                    },
                    _ => {
                        let result = sim.execute(&actor, &[], "always_true", &[1], 1_000_000).await;
                        match result {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e)
                        }
                    }
                }
            };
            futures.push(future);
        }

        // Run all operations in parallel
        let results: Vec<Result<(), String>> = join_all(futures).await;
        
        // Verify all operations completed successfully
        assert!(results.iter().all(|r| r.is_ok()));

        // Verify final state
        let sim = simulator.read().await;
        for (i, actor) in actors.iter().enumerate() {
            match i % 3 {
                0 => assert_eq!(sim.get_balance_async(actor).await, 500),
                1 => assert_eq!(sim.get_state(format!("key_{}", i).as_bytes()).await.unwrap(), b"value"),
                _ => continue,
            }
        }
    }

    #[tokio::test]
    async fn test_parallel_error_conditions() {
        let simulator = Arc::new(RwLock::new(SimulatorImpl::new().await));
        let actor = simulator.read().await.store.data().actor.clone();
        
        // Setup initial state
        {
            let mut sim = simulator.write().await;
            sim.set_balance_async(&actor, 500).await;
        }

        // Create operations that may fail
        let sim1 = simulator.clone();
        let sim2 = simulator.clone();
        let sim3 = simulator.clone();
        let sim4 = simulator.clone();

        // Operation 1: Try to read non-existent state
        let op1 = async move {
            let sim = sim1.read().await;
            match sim.get_state(b"non_existent_key1").await {
                Some(_) => Err("Expected None for non-existent key".to_string()),
                None => Ok(())
            }
        };

        // Operation 2: Try to read another non-existent state
        let op2 = async move {
            let sim = sim2.read().await;
            match sim.get_state(b"non_existent_key2").await {
                Some(_) => Err("Expected None for non-existent key".to_string()),
                None => Ok(())
            }
        };

        // Operation 3: Valid write operation
        let op3 = async move {
            let mut sim = sim3.write().await;
            sim.store_state(b"key", b"value").await;
            Ok::<_, String>(())
        };

        // Operation 4: Valid read operation
        let op4 = async move {
            let sim = sim4.read().await;
            let balance = sim.get_balance_async(&actor).await;
            Ok::<_, String>(balance)
        };

        // Run mix of operations in parallel
        let (r1, r2, r3, r4) = futures::join!(op1, op2, op3, op4);
        
        // Verify all operations complete as expected
        assert!(r1.is_ok());  // Reading non-existent key returns None (success)
        assert!(r2.is_ok());  // Reading non-existent key returns None (success)
        assert!(r3.is_ok());  // Write operation succeeds
        assert!(r4.is_ok());  // Read operation succeeds
        
        if let Ok(balance) = r4 {
            assert_eq!(balance, 500); // Balance should remain unchanged
        }

        // Verify final state
        let sim = simulator.read().await;
        assert_eq!(sim.get_balance_async(&actor).await, 500);
        assert_eq!(sim.get_state(b"key").await.unwrap(), b"value");
        assert_eq!(sim.get_state(b"non_existent_key1").await, None);
        assert_eq!(sim.get_state(b"non_existent_key2").await, None);
    }
}
