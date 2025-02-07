// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}},
};

use tokio::sync::RwLock;
use wasmtime::{Engine, Store, Instance, Module, Linker, Config, Caller};

use crate::{
    events::{Event, EventLog},
    gas::{GasCounter, MAX_CALL_DEPTH},
    types::WasmlAddress,
};

#[async_trait::async_trait]
pub trait Simulator: Send + Sync {
    fn get_balance<'a>(&'a self, account: &'a WasmlAddress) -> Pin<Box<dyn Future<Output = u64> + Send + 'a>>;
    fn set_balance<'a>(&'a mut self, account: &'a WasmlAddress, balance: u64) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
    fn store_state<'a>(&'a mut self, key: &'a [u8], value: &'a [u8]) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
    fn get_state<'a>(&'a self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>>;
    fn delete_state<'a>(&'a mut self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>>;
    fn execute<'a>(
        &'a mut self,
        actor: &'a WasmlAddress,
        target: &'a [u8],
        method: &'a str,
        args: &'a [u8],
        gas: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send + 'a>>;
    fn remaining_fuel(&self) -> u64;
    fn get_events(&self) -> Vec<Event>;
}

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

pub struct SimulatorImpl {
    pub balances: Arc<RwLock<HashMap<WasmlAddress, u64>>>,
    pub state: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    pub remaining_gas: Arc<RwLock<u64>>,
    pub store: Store<SimulatorState>,
    pub linker: Arc<Linker<SimulatorState>>,
    pub event_log: Arc<RwLock<EventLog>>,
    pub instance: wasmtime::Instance,
}

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

#[async_trait::async_trait]
impl Simulator for SimulatorImpl {
    fn get_balance<'a>(&'a self, account: &'a WasmlAddress) -> Pin<Box<dyn Future<Output = u64> + Send + 'a>> {
        let balances = self.balances.clone();
        Box::pin(async move {
            balances.read().await.get(account).copied().unwrap_or(0)
        })
    }

    fn set_balance<'a>(&'a mut self, account: &'a WasmlAddress, balance: u64) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
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

    fn remaining_fuel(&self) -> u64 {
        futures::executor::block_on(async {
            *self.remaining_gas.read().await
        })
    }

    fn get_events(&self) -> Vec<Event> {
        futures::executor::block_on(async {
            self.event_log.read().await.events().iter().cloned().collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulator() {
        let mut simulator = SimulatorImpl::new().await;
        let actor = WasmlAddress::default();
        let balance: u64 = 100;

        // Test balance operations
        let result = simulator.set_balance(&actor, balance).await;
        assert_eq!(result, ());

        let result = simulator.get_balance(&actor).await;
        assert_eq!(result, balance);

        // Test state operations
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        let result = simulator.store_state(&key, &value).await;
        assert_eq!(result, ());

        let result = simulator.get_state(&key).await;
        assert_eq!(result, Some(value.clone()));

        let result = simulator.delete_state(&key).await;
        assert_eq!(result, Some(value));

        let result = simulator.get_state(&key).await;
        assert_eq!(result, None);

        // Test execute
        let args = vec![1u8; 1]; // Allocate 1 byte to avoid zero allocation
        let result = simulator.execute(&actor, &[], "always_true", &args, 1_000_000).await;
        assert!(result.is_ok());
        
        // Test remaining fuel
        assert_eq!(simulator.remaining_fuel(), 0);
    }
}
