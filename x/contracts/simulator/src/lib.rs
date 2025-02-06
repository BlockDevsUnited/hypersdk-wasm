#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::collections::HashMap;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::sync::{Arc, Mutex};
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::str::FromStr;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use wasmtime::{Config, Engine, Linker, Module, Store, Val, ValType, FuncType, Caller, AsContext, AsContextMut};

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use thiserror::Error;

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[derive(Debug, Error)]
pub enum SimulatorError {
    #[error("{0}")]
    ContractExecution(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
    #[error("WASM error: {0}")]
    Wasm(#[from] wasmtime::Error),
    #[error("Memory access error: {0}")]
    Memory(#[from] wasmtime::MemoryAccessError),
    #[error("Memory not found")]
    MemoryNotFound,
}

#[derive(Clone, Debug)]
pub struct Address(pub Vec<u8>);

impl Address {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn new(bytes: Vec<u8>) -> Self {
        Address(bytes)
    }
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl FromStr for Address {
    type Err = SimulatorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Address(s.as_bytes().to_vec()))
    }
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[derive(Default)]
pub struct SimulatorState {
    state: HashMap<Vec<u8>, Vec<u8>>,
    balances: HashMap<Vec<u8>, u64>,
    contracts: HashMap<Vec<u8>, Vec<u8>>,
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl SimulatorState {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            balances: HashMap::new(),
            contracts: HashMap::new(),
        }
    }

    pub fn get_value(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.state.get(key).cloned()
    }

    pub fn set_value(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.state.insert(key, value);
    }
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[derive(Default)]
pub struct Simulator {
    engine: Engine,
    store: Store<()>,
    state: Arc<Mutex<SimulatorState>>,
    result: Option<Vec<u8>>,
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl Simulator {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.wasm_multi_value(true);
        config.wasm_multi_memory(true);
        config.async_support(true);
        
        let engine = Engine::new(&config).unwrap();
        let store = Store::new(&engine, ());
        let state = Arc::new(Mutex::new(SimulatorState::new()));
        
        Self {
            engine,
            store,
            state,
            result: None,
        }
    }

    pub fn with_state(state: Arc<Mutex<SimulatorState>>) -> Self {
        let mut config = Config::new();
        config.wasm_multi_value(true);
        config.wasm_multi_memory(true);
        config.async_support(true);
        
        let engine = Engine::new(&config).unwrap();
        let store = Store::new(&engine, ());
        
        Self {
            engine,
            store,
            state,
            result: None,
        }
    }

    pub fn get_state(&self) -> Arc<Mutex<SimulatorState>> {
        self.state.clone()
    }

    pub async fn execute(&mut self, code: &[u8], method: &str, params: &[u8], _gas: u64) -> Result<Vec<u8>, SimulatorError> {
        // Reset result
        self.result = None;
        
        // Create module from WASM bytecode
        let module = Module::new(&self.engine, code)?;
        
        // Create linker and add imports
        let mut linker = Linker::new(&self.engine);
        
        // Add contract module imports
        let result = Arc::new(Mutex::new(None));
        let result_clone = result.clone();
        
        linker.func_wrap("contract", "set_call_result", move |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
            let memory = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| SimulatorError::MemoryNotFound)?;

            let mut data = vec![0u8; len as usize];
            memory.read(caller.as_context_mut(), ptr as usize, &mut data)?;
            
            *result_clone.lock().unwrap() = Some(data);
            Ok(())
        })?;

        // Add input functions
        let params = params.to_vec();
        let params_len = params.len();
        linker.func_wrap("contract", "get_input_len", move || {
            Ok(params_len as i32)
        })?;

        let params_clone = params.clone();
        linker.func_wrap("contract", "get_input", move |mut caller: Caller<'_, ()>, ptr: i32| {
            let memory = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| SimulatorError::MemoryNotFound)?;

            memory.write(caller.as_context_mut(), ptr as usize, &params_clone)?;
            Ok(())
        })?;

        // Add state module imports
        let state = self.state.clone();
        linker.func_wrap("state", "get", move |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
            let memory = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| SimulatorError::MemoryNotFound)?;

            let mut data = vec![0u8; len as usize];
            memory.read(caller.as_context_mut(), ptr as usize, &mut data)?;
            
            let value = state.lock().unwrap().get_value(&data);
            
            if let Some(value) = value {
                let ptr = memory.data_size(caller.as_context()) as i32;
                memory.grow(caller.as_context_mut(), 1)?;
                memory.write(caller.as_context_mut(), ptr as usize, &value)?;
                Ok(ptr)
            } else {
                Ok(-1)
            }
        })?;

        let state = self.state.clone();
        linker.func_wrap("state", "put", move |mut caller: Caller<'_, ()>, key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32| {
            let memory = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| SimulatorError::MemoryNotFound)?;

            let mut key = vec![0u8; key_len as usize];
            memory.read(caller.as_context_mut(), key_ptr as usize, &mut key)?;
            
            let mut value = vec![0u8; value_len as usize];
            memory.read(caller.as_context_mut(), value_ptr as usize, &mut value)?;
            
            state.lock().unwrap().set_value(key, value);
            Ok(())
        })?;

        // Add host imports
        linker.func_wrap(
            "host",
            "get_actor",
            |_: Caller<'_, ()>| {
                Ok(0i64)
            },
        )?;

        linker.func_wrap(
            "host",
            "get_height",
            |_: Caller<'_, ()>| {
                Ok(0i64)
            },
        )?;

        linker.func_wrap(
            "host",
            "get_timestamp",
            |_: Caller<'_, ()>| {
                Ok(0i64)
            },
        )?;

        // Create store and instantiate module
        let mut store = Store::new(&self.engine, ());
        let instance = linker.instantiate_async(&mut store, &module).await?;
        instance.get_typed_func::<(), ()>(&mut store, method)?.call_async(&mut store, ()).await?;

        // Get the result before dropping anything
        let final_result = result.lock().unwrap().take().unwrap_or_default();
        Ok(final_result)
    }

    pub fn get_balance(&self, account: Address) -> u64 {
        self.state.lock().unwrap().balances.get(&account.0).copied().unwrap_or_default()
    }

    pub fn set_balance(&mut self, account: Address, balance: u64) {
        let mut state = self.state.lock().unwrap();
        state.balances.insert(account.0, balance);
    }

    pub fn create_contract(&mut self, contract: Address, code: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        state.contracts.insert(contract.0, code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_creation() {
        let simulator = Simulator::new();
        assert!(simulator.get_state().lock().unwrap().get_value(&[]).is_none());
    }

    #[test]
    fn test_contract_creation() {
        let mut simulator = Simulator::new();
        let contract = Address::new(vec![1, 2, 3]);
        let code = vec![0, 1, 2, 3];
        simulator.create_contract(contract.clone(), code.clone());
        assert_eq!(simulator.get_state().lock().unwrap().contracts.get(&contract.0).unwrap(), &code);
    }

    #[test]
    fn test_balance() {
        let mut simulator = Simulator::new();
        let account = Address::new(vec![1, 2, 3]);
        simulator.set_balance(account.clone(), 100);
        assert_eq!(simulator.get_balance(account), 100);
    }
}

// For wasm32 target, provide dummy types
#[cfg(target_arch = "wasm32")]
pub struct SimulatorState;

#[cfg(target_arch = "wasm32")]
pub struct Simulator;

#[cfg(target_arch = "wasm32")]
impl Simulator {
    pub fn new() -> Self {
        Self
    }
}
