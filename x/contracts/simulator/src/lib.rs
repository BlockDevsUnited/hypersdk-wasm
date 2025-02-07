#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::collections::HashMap;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::sync::Arc;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use tokio::sync::RwLock;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::str::FromStr;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use wasmtime::{Config, Engine, Linker, Module, Store, Caller, AsContextMut};

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

#[derive(Debug, Clone, Default)]
pub struct Address(pub Vec<u8>);

impl Address {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn new(bytes: Vec<u8>) -> Self {
        Address(bytes)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
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
    state: Arc<RwLock<SimulatorState>>,
    result: Option<Vec<u8>>,
    actor: Address,
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl Simulator {
    pub async fn new(actor: Address) -> Self {
        let mut config = Config::new();
        config.wasm_multi_value(true);
        config.wasm_multi_memory(true);
        config.async_support(true);
        
        let engine = Engine::new(&config).unwrap();
        let store = Store::new(&engine, ());
        let state = Arc::new(RwLock::new(SimulatorState::new()));
        
        Self {
            engine,
            store,
            state,
            result: None,
            actor,
        }
    }

    pub async fn with_state(state: Arc<RwLock<SimulatorState>>) -> Self {
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
            actor: Address::default(),
        }
    }

    pub async fn get_state(&self) -> Arc<RwLock<SimulatorState>> {
        self.state.clone()
    }

    pub async fn execute(&mut self, code: &[u8], method: &str, params: &[u8], _gas: u64) -> Result<Vec<u8>, SimulatorError> {
        // Reset result
        self.result = None;
        
        // Get contract code from state
        let state = self.state.read().await;
        let contract_code = state.contracts.get(code).cloned().unwrap_or_else(|| code.to_vec());
        drop(state);
        
        // Create module from WASM bytecode
        let module = Module::new(&self.engine, contract_code)?;
        
        // Create linker and add imports
        let mut linker = Linker::new(&self.engine);
        
        // Add contract module imports
        let result = Arc::new(tokio::sync::Mutex::new(None));
        let result_clone = result.clone();
        
        linker.func_wrap("contract", "set_call_result", move |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
            let memory = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| SimulatorError::MemoryNotFound)?;

            let mut data = vec![0u8; len as usize];
            memory.read(caller.as_context_mut(), ptr as usize, &mut data)?;
            
            let result_clone2 = result_clone.clone();
            let data_clone = data.clone();
            tokio::spawn(async move {
                *result_clone2.lock().await = Some(data_clone);
            });
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

            let mut key = vec![0u8; len as usize];
            memory.read(caller.as_context_mut(), ptr as usize, &mut key)?;

            let state_clone = state.clone();
            let key_clone = key.clone();
            
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let state = state_clone.read().await;
                    if let Some(value) = state.get_value(&key_clone) {
                        memory.write(caller.as_context_mut(), ptr as usize, &value)?;
                    }
                    Ok::<_, SimulatorError>(())
                })
            })?;
            Ok(())
        })?;

        let state = self.state.clone();
        linker.func_wrap("state", "set", move |mut caller: Caller<'_, ()>, key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32| {
            let memory = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| SimulatorError::MemoryNotFound)?;

            let mut key = vec![0u8; key_len as usize];
            let mut value = vec![0u8; value_len as usize];
            memory.read(caller.as_context_mut(), key_ptr as usize, &mut key)?;
            memory.read(caller.as_context_mut(), value_ptr as usize, &mut value)?;

            let state_clone = state.clone();
            let key_clone = key.clone();
            let value_clone = value.clone();
            
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let mut state = state_clone.write().await;
                    state.set_value(key_clone, value_clone);
                })
            });
            Ok(())
        })?;

        // Get instance and run
        let instance = linker.instantiate(&mut self.store, &module)?;
        let run = instance.get_typed_func::<(), ()>(&mut self.store, method)?;
        run.call_async(&mut self.store, ()).await?;

        // Get result
        let final_result = result.lock().await.take().unwrap_or_default();
        Ok(final_result)
    }

    pub async fn get_balance(&self, account: Address) -> u64 {
        let state = self.state.read().await;
        *state.balances.get(account.as_bytes()).unwrap_or(&0)
    }

    pub async fn set_balance(&mut self, account: Address, balance: u64) {
        let mut state = self.state.write().await;
        state.balances.insert(account.as_bytes().to_vec(), balance);
    }

    pub async fn create_contract(&mut self, address: Vec<u8>, code: Vec<u8>) -> Result<(), SimulatorError> {
        let mut state = self.state.write().await;
        state.contracts.insert(address, code);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulator_creation() {
        let actor = Address::new(vec![0; 32]);
        let _simulator = Simulator::new(actor).await;
    }

    #[tokio::test]
    async fn test_contract_creation() {
        let actor = Address::new(vec![0; 32]);
        let mut simulator = Simulator::new(actor).await;
        let address = vec![1; 32];
        let code = vec![0, 1, 2, 3];
        simulator.create_contract(address.clone(), code.clone()).await.unwrap();
    }

    #[tokio::test]
    async fn test_balance() {
        let actor = Address::new(vec![0; 32]);
        let mut simulator = Simulator::new(actor.clone()).await;
        simulator.set_balance(actor.clone(), 100).await;
        assert_eq!(simulator.get_balance(actor).await, 100);
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
