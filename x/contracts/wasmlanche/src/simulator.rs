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
    host::Host,
    error::Error,
};

#[async_trait::async_trait]
pub trait Simulator: Host {
    async fn execute(
        &mut self,
        actor: &WasmlAddress,
        _target: &[u8], // Ignore target for now
        method: &str,
        args: &[u8],
        gas: u64,
    ) -> Result<Vec<u8>, String>;
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
        
        linker.func_wrap("env", "db_read", |mut caller: Caller<'_, SimulatorState>, key_ptr: i32, key_len: i32| -> i32 {
            // Get key from memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut key = vec![0u8; key_len as usize];
            memory.read(&caller, key_ptr as usize, &mut key).unwrap();
            
            // Read from state
            let state = caller.data().state.clone();
            let value = futures::executor::block_on(async move {
                state.read().await.get(&key).cloned()
            });
            
            match value {
                Some(value) => {
                    // Allocate memory for value and copy it
                    let current_ptr = caller.data().next_ptr.fetch_add(value.len() as u64, Ordering::SeqCst);
                    memory.write(&mut caller, current_ptr as usize, &value).unwrap();
                    current_ptr as i32
                }
                None => -1,
            }
        })
        .expect("Failed to define db_read function");

        linker.func_wrap("env", "abort", |_caller: Caller<'_, SimulatorState>, _ptr: i32| {
            panic!("Contract aborted");
        })
        .expect("Failed to define abort function");

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

        linker.func_wrap("env", "allocate", move |mut caller: Caller<'_, SimulatorState>, _context: i32, data_ptr: i32, size: i32| -> i32 { 
            if size <= 0 {
                panic!("failed to allocate memory");
            }

            // Get next available memory pointer
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
            
            // Copy data if source pointer is non-zero and within valid memory bounds
            if data_ptr != 0 {
                let data_size = match caller.data().allocation_sizes.lock().unwrap().get(&data_ptr) {
                    Some(&s) => s,
                    None => panic!("Invalid source pointer"),
                };
                
                if data_size < size {
                    panic!("Source buffer too small");
                }
                
                let mut data = vec![0u8; size as usize];
                memory.read(&caller, data_ptr as usize, &mut data).unwrap();
                memory.write(&mut caller, current_ptr as usize, &data).unwrap();
            } else {
                // Initialize memory to zero if no source data
                let zeros = vec![0u8; size as usize];
                memory.write(&mut caller, current_ptr as usize, &zeros).unwrap();
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
                (func $host_allocate (import "env" "allocate") (param i32 i32 i32) (result i32))
                (func $host_always_true (import "env" "always_true") (param i32) (result i32))
                (func $host_allocate_context (import "env" "allocate_context") (param i32) (result i32))
                (func $host_highest_allocated_address (import "env" "highest_allocated_address") (param i32) (result i32))
                (func $host_combine_last_bit_of_each_id_byte (import "env" "combine_last_bit_of_each_id_byte") (param i32) (result i32))
                (func $host_db_read (import "env" "db_read") (param i32 i32) (result i32))

                ;; Memory section
                (memory (export "memory") 1)

                ;; Global section
                (global $g0 (mut i32) (i32.const 65536))
                (export "__heap_base" (global $g0))

                ;; Exported functions that use host functions
                (func (export "allocate") (param i32 i32 i32) (result i32)
                    ;; Call host allocate and return result
                    local.get 0
                    local.get 1
                    local.get 2
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

                (func (export "combine_last_bit_of_each_id_byte") (param i32) (result i32)
                    ;; Call host combine_last_bit_of_each_id_byte and return result
                    local.get 0
                    call $host_combine_last_bit_of_each_id_byte
                )

                (func (export "db_read") (param i32 i32) (result i32)
                    ;; Call host db_read and return result
                    local.get 0
                    local.get 1
                    call $host_db_read
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

    fn add_host_functions(&self, linker: &mut Linker<SimulatorState>) -> Result<(), Error> {
        // Add host functions to the linker
        linker.func_wrap("env", "db_read", |mut caller: Caller<'_, SimulatorState>, key_ptr: i32, key_len: i32| -> i32 {
            // Get key from memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut key = vec![0u8; key_len as usize];
            memory.read(&caller, key_ptr as usize, &mut key).unwrap();
            
            // Read from state
            let state = caller.data().state.clone();
            let value = futures::executor::block_on(async move {
                state.read().await.get(&key).cloned()
            });
            
            match value {
                Some(value) => {
                    // Allocate memory for value and copy it
                    let current_ptr = caller.data().next_ptr.fetch_add(value.len() as u64, Ordering::SeqCst);
                    memory.write(&mut caller, current_ptr as usize, &value).unwrap();
                    current_ptr as i32
                }
                None => -1,
            }
        })
        .expect("Failed to define db_read function");

        linker.func_wrap("env", "abort", |_caller: Caller<'_, SimulatorState>, _ptr: i32| {
            panic!("Contract aborted");
        })
        .expect("Failed to define abort function");

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

        linker.func_wrap("env", "allocate", move |mut caller: Caller<'_, SimulatorState>, _context: i32, data_ptr: i32, size: i32| -> i32 { 
            if size <= 0 {
                panic!("failed to allocate memory");
            }

            // Get next available memory pointer
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
            
            // Copy data if source pointer is non-zero and within valid memory bounds
            if data_ptr != 0 {
                let data_size = match caller.data().allocation_sizes.lock().unwrap().get(&data_ptr) {
                    Some(&s) => s,
                    None => panic!("Invalid source pointer"),
                };
                
                if data_size < size {
                    panic!("Source buffer too small");
                }
                
                let mut data = vec![0u8; size as usize];
                memory.read(&caller, data_ptr as usize, &mut data).unwrap();
                memory.write(&mut caller, current_ptr as usize, &data).unwrap();
            } else {
                // Initialize memory to zero if no source data
                let zeros = vec![0u8; size as usize];
                memory.write(&mut caller, current_ptr as usize, &zeros).unwrap();
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

        Ok(())
    }
}

#[async_trait::async_trait]
impl Simulator for SimulatorImpl {
    async fn execute(
        &mut self,
        actor: &WasmlAddress,
        _target: &[u8], // Ignore target for now
        method: &str,
        args: &[u8],
        gas: u64,
    ) -> Result<Vec<u8>, String> {
        println!("Executing method: {}", method);
        
        // Initialize actor and gas counter
        self.store.data_mut().actor = actor.clone();
        self.store.data_mut().gas_counter = Some(GasCounter::new(gas));
        
        if method == "deploy" {
            // For deploy, we need to compile and instantiate the WASM module
            let mut config = Config::new();
            config.async_support(true);
            let engine = Engine::new(&config).map_err(|e| e.to_string())?;
            let module = Module::new(&engine, args).map_err(|e| e.to_string())?;
            let mut linker = Linker::new(&engine);
            
            // Add host functions to the linker
            self.add_host_functions(&mut linker).map_err(|e| e.to_string())?;
            
            // Instantiate the module
            self.instance = linker.instantiate(&mut self.store, &module)
                .map_err(|e| e.to_string())?;
            
            Ok(Vec::new())
        } else if method == "allocate_context" {
            if args.len() != 4 {
                return Err("allocate_context requires a 4-byte size parameter".to_string());
            }
            let size = i32::from_le_bytes(args.try_into().unwrap());
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            println!("Got function {}", method);
            let func_typed = func.typed::<i32, i32>(&self.store)
                .map_err(|e| e.to_string())?;
            println!("Typed function");
            let result_ptr = func_typed.call_async(&mut self.store, size)
                .await
                .map_err(|e| {
                    println!("Function call failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function: {}", result_ptr);
            
            // For allocate functions, return the pointer directly
            Ok((result_ptr as i32).to_le_bytes().to_vec())
        } else if method == "allocate" {
            // For allocate, we pass the data directly to the function
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            println!("Got function {}", method);

            // Call allocate with context and data
            let func_typed = func.typed::<(i32, i32, i32), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            println!("Typed function");

            // Get context pointer and data pointer from args
            if args.len() < 12 {
                return Err("allocate requires 12 bytes for context pointer, data pointer, and size".to_string());
            }
            let context_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let data_ptr = i32::from_le_bytes(args[4..8].try_into().unwrap());
            let size = i32::from_le_bytes(args[8..12].try_into().unwrap());

            println!("Allocating with context_ptr={}, data_ptr={}, size={}", context_ptr, data_ptr, size);

            // Call allocate with context pointer and data
            let result_ptr = func_typed.call_async(&mut self.store, (context_ptr, data_ptr, size))
                .await
                .map_err(|e| {
                    println!("Allocation failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function: {}", result_ptr);

            // Verify the allocation was successful
            if result_ptr < 0 {
                let err = format!("allocation failed with result {}", result_ptr);
                println!("{}", err);
                return Err(err);
            }

            // Track this allocation in our system
            self.store.data_mut().allocation_sizes.lock().unwrap().insert(result_ptr, size);

            // For allocate functions, return the pointer directly
            Ok(result_ptr.to_le_bytes().to_vec())
        } else if method == "db_read" {
            if args.len() < 8 {
                return Err("db_read requires 8 bytes for key pointer and length".to_string());
            }
            let key_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let key_len = i32::from_le_bytes(args[4..8].try_into().unwrap());

            println!("Reading from db with key_ptr={}, key_len={}", key_ptr, key_len);

            // Call db_read with key pointer and length
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(i32, i32), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            let result_ptr = func_typed.call_async(&mut self.store, (key_ptr, key_len))
                .await
                .map_err(|e| {
                    println!("DB read failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function: {}", result_ptr);

            // Verify the read was successful
            if result_ptr < 0 {
                let err = format!("db read failed with result {}", result_ptr);
                println!("{}", err);
                return Err(err);
            }

            // For db_read, return the pointer directly
            Ok(result_ptr.to_le_bytes().to_vec())
        } else {
            Err(format!("unknown method: {}", method))
        }
    }
}

impl Host for SimulatorImpl {
    fn store_state(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        let state = self.state.clone();
        futures::executor::block_on(async move {
            state.write().await.insert(key.to_vec(), value.to_vec());
            Ok(())
        })
    }

    fn get_state(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let state = self.state.clone();
        futures::executor::block_on(async move {
            Ok(state.read().await.get(key).cloned())
        })
    }

    fn delete_state(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let state = self.state.clone();
        futures::executor::block_on(async move {
            Ok(state.write().await.remove(key))
        })
    }

    fn get_events(&self) -> Vec<Event> {
        futures::executor::block_on(async {
            self.event_log.read().await.events().iter().cloned().collect()
        })
    }

    fn add_event(&mut self, event: Event) -> Result<(), Error> {
        let event_log = self.event_log.clone();
        futures::executor::block_on(async move {
            event_log.write().await.add_event(event);
            Ok(())
        })
    }

    fn charge_gas(&mut self, amount: u64) -> Result<(), Error> {
        if let Some(gas_counter) = &mut self.store.data_mut().gas_counter {
            gas_counter.charge_gas(amount)
        } else {
            Err(Error::OutOfGas("No gas counter initialized"))
        }
    }

    fn remaining_gas(&self) -> u64 {
        if let Some(gas_counter) = &self.store.data().gas_counter {
            gas_counter.gas_remaining()
        } else {
            0
        }
    }

    fn get_balance(&self, account: &WasmlAddress) -> u64 {
        let balances = self.balances.clone();
        futures::executor::block_on(async move {
            balances.read().await.get(account).copied().unwrap_or(0)
        })
    }

    fn set_balance(&mut self, account: &WasmlAddress, amount: u64) {
        let balances = self.balances.clone();
        futures::executor::block_on(async move {
            balances.write().await.insert(account.clone(), amount);
        })
    }

    fn emit_event(&mut self, event: Event) -> Result<(), Error> {
        self.add_event(event)
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
        simulator.set_balance(&actor, balance);
        assert_eq!(simulator.get_balance(&actor), balance);

        // Test state operations
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        let result = simulator.store_state(&key, &value);
        assert!(result.is_ok());

        let result = simulator.get_state(&key);
        assert_eq!(result.unwrap(), Some(value.clone()));

        let result = simulator.delete_state(&key);
        assert_eq!(result.unwrap(), Some(value));

        let result = simulator.get_state(&key);
        assert_eq!(result.unwrap(), None);

        // Test execute with allocate_context
        let result = simulator.execute(&actor, b"target", "allocate_context", &[1, 0, 0, 0], 1000).await;
        assert!(result.is_ok());
        let context_ptr = i32::from_le_bytes(result.unwrap().try_into().unwrap());

        // First allocate memory for test data
        let test_data = b"test_data".to_vec();
        println!("Test data: {:?}", test_data);
        let mut alloc_args = context_ptr.to_le_bytes().to_vec();
        alloc_args.extend_from_slice(&[0, 0, 0, 0]); // No source data pointer
        alloc_args.extend_from_slice(&(test_data.len() as i32).to_le_bytes());
        println!("First allocation args: context_ptr={}, data_ptr=0, size={}", context_ptr, test_data.len());
        
        let result = simulator.execute(&actor, b"target", "allocate", &alloc_args, 1000).await;
        if let Err(e) = &result {
            println!("First allocation failed: {}", e);
        }
        assert!(result.is_ok());
        let data_ptr = i32::from_le_bytes(result.unwrap().try_into().unwrap());
        println!("First allocation succeeded, got pointer: {}", data_ptr);

        // Write test data to allocated memory
        let memory = simulator.instance.get_memory(&mut simulator.store, "memory").unwrap();
        memory.write(&mut simulator.store, data_ptr as usize, &test_data).unwrap();
        println!("Wrote test data to memory at {}", data_ptr);

        // Track this allocation in our system
        simulator.store.data_mut().allocation_sizes.lock().unwrap().insert(data_ptr, test_data.len() as i32);
        println!("Tracked allocation in system: ptr={}, size={}", data_ptr, test_data.len());

        // Read back the data to verify it was written correctly
        let mut buffer = vec![0u8; test_data.len()];
        memory.read(&mut simulator.store, data_ptr as usize, &mut buffer).unwrap();
        println!("Read back data from first allocation: {:?}", buffer);
        assert_eq!(buffer, test_data, "Initial data write failed");

        // Now allocate new memory and copy from the previous allocation
        let mut args = context_ptr.to_le_bytes().to_vec();
        args.extend_from_slice(&data_ptr.to_le_bytes());
        args.extend_from_slice(&(test_data.len() as i32).to_le_bytes());
        println!("Second allocation args: context_ptr={}, data_ptr={}, size={}", context_ptr, data_ptr, test_data.len());

        // Call allocate with the existing data
        let result = simulator.execute(&actor, b"target", "allocate", &args, 1000).await;
        if let Err(e) = &result {
            println!("Second allocation failed: {}", e);
        }
        assert!(result.is_ok(), "Second allocation failed: {:?}", result);
        let new_data_ptr = i32::from_le_bytes(result.unwrap().try_into().unwrap());
        println!("Second allocation succeeded, got pointer: {}", new_data_ptr);

        // Verify the data was copied correctly by reading from the new location
        let mut new_buffer = vec![0u8; test_data.len()];
        memory.read(&mut simulator.store, new_data_ptr as usize, &mut new_buffer).unwrap();
        println!("Read back data from second allocation: {:?}", new_buffer);
        assert_eq!(new_buffer, test_data, "Data copy failed");
        
        // Test gas
        let result = simulator.charge_gas(500);
        assert!(result.is_ok());
        assert_eq!(simulator.remaining_gas(), 500);
        
        // Test events
        let event = Event::new("test", "data");
        let result = simulator.add_event(event.clone());
        assert!(result.is_ok());
        assert_eq!(simulator.get_events(), vec![event]);
    }
}
