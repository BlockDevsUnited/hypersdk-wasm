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
    pub scan_keys: Arc<Mutex<Option<Vec<Vec<u8>>>>>,  // Track keys for db_scan
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
    pub async fn new() -> Result<Self, Error> {
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
                scan_keys: Arc::new(Mutex::new(None)),
            },
        );

        let mut linker = Linker::new(&engine);
        
        let module = Module::new(&engine, r#"
            (module
                (memory (export "memory") 1)
            )
        "#).expect("Failed to create module");
        
        let instance = linker.instantiate_async(&mut store, &module).await.expect("Failed to instantiate module");

        let simulator = SimulatorImpl {
            balances: balances.clone(),
            state: state.clone(),
            remaining_gas: remaining_gas.clone(),
            store,
            instance,
            linker: Arc::new(linker),
            event_log: event_log.clone(),
        };

        Ok(simulator)
    }

    fn add_host_functions(&self, linker: &mut Linker<SimulatorState>) -> Result<(), Error> {
        // Add host functions to the linker
        linker.func_wrap("env", "db_write", |mut caller: Caller<'_, SimulatorState>, key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32| {
            // Get key and value from memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut key = vec![0u8; key_len as usize];
            let mut value = vec![0u8; value_len as usize];
            memory.read(&caller, key_ptr as usize, &mut key).unwrap();
            memory.read(&caller, value_ptr as usize, &mut value).unwrap();
            
            // Write to state
            let state = caller.data().state.clone();
            futures::executor::block_on(async move {
                state.write().await.insert(key, value);
            });
        })
        .expect("Failed to define db_write function");

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

        linker.func_wrap("env", "db_scan", |mut caller: Caller<'_, SimulatorState>, start_ptr: i32, start_len: i32, end_ptr: i32, end_len: i32| -> i32 {
            // Get start and end keys from memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut start_key = vec![0u8; start_len as usize];
            let mut end_key = vec![0u8; end_len as usize];
            memory.read(&caller, start_ptr as usize, &mut start_key).unwrap();
            memory.read(&caller, end_ptr as usize, &mut end_key).unwrap();
            
            // Get state and scan for matching keys
            let state = caller.data().state.clone();
            let mut matching_keys = futures::executor::block_on(async move {
                let state = state.read().await;
                state.iter()
                    .filter(|(k, _)| *k >= &start_key && *k <= &end_key)
                    .map(|(k, _)| k.clone())
                    .collect::<Vec<_>>()
            });

            // Sort keys to ensure deterministic ordering
            matching_keys.sort();

            // Store the keys in the state for iteration
            *caller.data().scan_keys.lock().unwrap() = Some(matching_keys);
            
            // Return the number of matching keys
            caller.data().scan_keys.lock().unwrap().as_ref().unwrap().len() as i32
        })
        .expect("Failed to define db_scan function");

        linker.func_wrap("env", "db_next", |mut caller: Caller<'_, SimulatorState>| -> i32 {
            // Get the next key
            let key = {
                let mut scan_keys = caller.data().scan_keys.lock().unwrap();
                let keys = scan_keys.as_mut().expect("db_next called without db_scan");
                
                if keys.is_empty() {
                    return -1;
                }
                
                keys.remove(0)
            };
            
            // Now that we've released the lock, we can use memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            
            // Allocate memory for the key
            let current_ptr = caller.data().next_ptr.fetch_add(key.len() as u64, Ordering::SeqCst);
            memory.write(&mut caller, current_ptr as usize, &key).unwrap();
            
            current_ptr as i32
        })
        .expect("Failed to define db_next function");

        linker.func_wrap("env", "db_remove", |mut caller: Caller<'_, SimulatorState>, key_ptr: i32, key_len: i32| {
            // Get key from memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut key = vec![0u8; key_len as usize];
            memory.read(&caller, key_ptr as usize, &mut key).unwrap();
            
            // Remove from state
            let state = caller.data().state.clone();
            futures::executor::block_on(async move {
                state.write().await.remove(&key);
            });
        })
        .expect("Failed to define db_remove function");

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

        linker.func_wrap("env", "addr_canonicalize", |mut caller: Caller<'_, SimulatorState>, addr_ptr: i32, addr_len: i32| -> i32 {
            // Get address from memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut addr = vec![0u8; addr_len as usize];
            memory.read(&caller, addr_ptr as usize, &mut addr).unwrap();
            
            // For now, we'll just return the same address as canonical
            // In a real implementation, this would do proper address canonicalization
            let current_ptr = caller.data().next_ptr.fetch_add(addr.len() as u64, Ordering::SeqCst);
            memory.write(&mut caller, current_ptr as usize, &addr).unwrap();
            
            // Store the allocation size
            caller.data().allocation_sizes.lock().unwrap().insert(current_ptr as i32, addr.len() as i32);
            
            current_ptr as i32
        })
        .expect("Failed to define addr_canonicalize function");

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

        linker.func_wrap("env", "addr_validate", |mut caller: Caller<'_, SimulatorState>, addr_ptr: i32, addr_len: i32| -> i32 {
            // Get address from memory
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut addr = vec![0u8; addr_len as usize];
            memory.read(&caller, addr_ptr as usize, &mut addr).unwrap();
            
            // Check if address is valid (32 bytes)
            if addr.len() != 32 {
                return -1;
            }
            
            // Return success
            0
        })
        .expect("Failed to define addr_validate function");

        Ok(())
    }

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
            
            // Instantiate the module asynchronously
            self.instance = linker.instantiate_async(&mut self.store, &module)
                .await.map_err(|e| e.to_string())?;
            
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
        } else if method == "db_write" {
            if args.len() < 16 {
                return Err("db_write requires 16 bytes for key pointer, key length, value pointer, and value length".to_string());
            }
            let key_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let key_len = i32::from_le_bytes(args[4..8].try_into().unwrap());
            let value_ptr = i32::from_le_bytes(args[8..12].try_into().unwrap());
            let value_len = i32::from_le_bytes(args[12..16].try_into().unwrap());

            println!("Writing to db with key_ptr={}, key_len={}, value_ptr={}, value_len={}", key_ptr, key_len, value_ptr, value_len);

            // Call db_write with key pointer, key length, value pointer, and value length
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(i32, i32, i32, i32), ()>(&self.store)
                .map_err(|e| e.to_string())?;
            func_typed.call_async(&mut self.store, (key_ptr, key_len, value_ptr, value_len))
                .await
                .map_err(|e| {
                    println!("DB write failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function");

            Ok(Vec::new())
        } else if method == "db_remove" {
            if args.len() < 8 {
                return Err("db_remove requires 8 bytes for key pointer and length".to_string());
            }
            let key_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let key_len = i32::from_le_bytes(args[4..8].try_into().unwrap());

            println!("Removing from db with key_ptr={}, key_len={}", key_ptr, key_len);

            // Call db_remove with key pointer and length
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(i32, i32), ()>(&self.store)
                .map_err(|e| e.to_string())?;
            func_typed.call_async(&mut self.store, (key_ptr, key_len))
                .await
                .map_err(|e| {
                    println!("DB remove failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function");

            Ok(Vec::new())
        } else if method == "db_scan" {
            if args.len() < 16 {
                return Err("db_scan requires 16 bytes for start key pointer, start key length, end key pointer, and end key length".to_string());
            }
            let start_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let start_len = i32::from_le_bytes(args[4..8].try_into().unwrap());
            let end_ptr = i32::from_le_bytes(args[8..12].try_into().unwrap());
            let end_len = i32::from_le_bytes(args[12..16].try_into().unwrap());

            println!("Scanning db with start_ptr={}, start_len={}, end_ptr={}, end_len={}", start_ptr, start_len, end_ptr, end_len);

            // Call db_scan with start key pointer, start key length, end key pointer, and end key length
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(i32, i32, i32, i32), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            let result_count = func_typed.call_async(&mut self.store, (start_ptr, start_len, end_ptr, end_len))
                .await
                .map_err(|e| {
                    println!("DB scan failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function: {}", result_count);

            Ok(result_count.to_le_bytes().to_vec())
        } else if method == "db_next" {
            println!("Getting next key from db scan");

            // Call db_next
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            let result_ptr = func_typed.call_async(&mut self.store, ())
                .await
                .map_err(|e| {
                    println!("DB next failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function: {}", result_ptr);

            Ok(result_ptr.to_le_bytes().to_vec())
        } else if method == "addr_validate" {
            if args.len() < 8 {
                return Err("addr_validate requires 8 bytes for address pointer and length".to_string());
            }
            let addr_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let addr_len = i32::from_le_bytes(args[4..8].try_into().unwrap());

            println!("Validating address with addr_ptr={}, addr_len={}", addr_ptr, addr_len);

            // Call addr_validate with address pointer and length
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(i32, i32), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            let result = func_typed.call_async(&mut self.store, (addr_ptr, addr_len))
                .await
                .map_err(|e| {
                    println!("Address validation failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function: {}", result);

            Ok(result.to_le_bytes().to_vec())
        } else if method == "test" {
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            let result = func_typed.call_async(&mut self.store, ())
                .await
                .map_err(|e| {
                    println!("Function call failed: {}", e);
                    e.to_string()
                })?;
            Ok(result.to_le_bytes().to_vec())
        } else {
            Err(format!("unknown method: {}", method))
        }
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
            
            // Instantiate the module asynchronously
            self.instance = linker.instantiate_async(&mut self.store, &module)
                .await.map_err(|e| e.to_string())?;
            
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
        } else if method == "db_write" {
            if args.len() < 16 {
                return Err("db_write requires 16 bytes for key pointer, key length, value pointer, and value length".to_string());
            }
            let key_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let key_len = i32::from_le_bytes(args[4..8].try_into().unwrap());
            let value_ptr = i32::from_le_bytes(args[8..12].try_into().unwrap());
            let value_len = i32::from_le_bytes(args[12..16].try_into().unwrap());

            println!("Writing to db with key_ptr={}, key_len={}, value_ptr={}, value_len={}", key_ptr, key_len, value_ptr, value_len);

            // Call db_write with key pointer, key length, value pointer, and value length
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(i32, i32, i32, i32), ()>(&self.store)
                .map_err(|e| e.to_string())?;
            func_typed.call_async(&mut self.store, (key_ptr, key_len, value_ptr, value_len))
                .await
                .map_err(|e| {
                    println!("DB write failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function");

            Ok(Vec::new())
        } else if method == "db_remove" {
            if args.len() < 8 {
                return Err("db_remove requires 8 bytes for key pointer and length".to_string());
            }
            let key_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let key_len = i32::from_le_bytes(args[4..8].try_into().unwrap());

            println!("Removing from db with key_ptr={}, key_len={}", key_ptr, key_len);

            // Call db_remove with key pointer and length
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(i32, i32), ()>(&self.store)
                .map_err(|e| e.to_string())?;
            func_typed.call_async(&mut self.store, (key_ptr, key_len))
                .await
                .map_err(|e| {
                    println!("DB remove failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function");

            Ok(Vec::new())
        } else if method == "db_scan" {
            if args.len() < 16 {
                return Err("db_scan requires 16 bytes for start key pointer, start key length, end key pointer, and end key length".to_string());
            }
            let start_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let start_len = i32::from_le_bytes(args[4..8].try_into().unwrap());
            let end_ptr = i32::from_le_bytes(args[8..12].try_into().unwrap());
            let end_len = i32::from_le_bytes(args[12..16].try_into().unwrap());

            println!("Scanning db with start_ptr={}, start_len={}, end_ptr={}, end_len={}", start_ptr, start_len, end_ptr, end_len);

            // Call db_scan with start key pointer, start key length, end key pointer, and end key length
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(i32, i32, i32, i32), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            let result_count = func_typed.call_async(&mut self.store, (start_ptr, start_len, end_ptr, end_len))
                .await
                .map_err(|e| {
                    println!("DB scan failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function: {}", result_count);

            Ok(result_count.to_le_bytes().to_vec())
        } else if method == "db_next" {
            println!("Getting next key from db scan");

            // Call db_next
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            let result_ptr = func_typed.call_async(&mut self.store, ())
                .await
                .map_err(|e| {
                    println!("DB next failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function: {}", result_ptr);

            Ok(result_ptr.to_le_bytes().to_vec())
        } else if method == "addr_validate" {
            if args.len() < 8 {
                return Err("addr_validate requires 8 bytes for address pointer and length".to_string());
            }
            let addr_ptr = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let addr_len = i32::from_le_bytes(args[4..8].try_into().unwrap());

            println!("Validating address with addr_ptr={}, addr_len={}", addr_ptr, addr_len);

            // Call addr_validate with address pointer and length
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(i32, i32), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            let result = func_typed.call_async(&mut self.store, (addr_ptr, addr_len))
                .await
                .map_err(|e| {
                    println!("Address validation failed: {}", e);
                    e.to_string()
                })?;
            println!("Called function: {}", result);

            Ok(result.to_le_bytes().to_vec())
        } else if method == "test" {
            let func = self.instance.get_func(&mut self.store, method)
                .ok_or_else(|| format!("function {} not found", method))?;
            let func_typed = func.typed::<(), i32>(&self.store)
                .map_err(|e| e.to_string())?;
            let result = func_typed.call_async(&mut self.store, ())
                .await
                .map_err(|e| {
                    println!("Function call failed: {}", e);
                    e.to_string()
                })?;
            Ok(result.to_le_bytes().to_vec())
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
    use crate::types::WasmlAddress;

    #[tokio::test]
    async fn test_simulator() {
        let mut simulator = SimulatorImpl::new().await.expect("Failed to create simulator");
        let actor = simulator.store.data().actor.clone();
        
        // Create a simple test module with memory export and test function
        let module = Module::new(simulator.store.engine(), r#"
            (module
                (memory (export "memory") 1)
                (func (export "test") (result i32)
                    i32.const 42
                )
            )
        "#).expect("Failed to create module");
        
        // Instantiate the module asynchronously
        let instance = simulator.linker.instantiate_async(
            &mut simulator.store,
            &module,
        ).await.expect("Failed to instantiate module");
        
        simulator.instance = instance;

        // Call the test function using the Simulator trait implementation
        let result = Simulator::execute(
            &mut simulator,
            &actor,
            &[],
            "test",
            &[],
            0,
        ).await;
        assert!(result.is_ok());
    }
}
