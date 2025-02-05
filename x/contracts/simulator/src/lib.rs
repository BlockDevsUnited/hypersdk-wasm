#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::collections::HashMap;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::sync::{Arc, RwLock};
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
use std::str::FromStr;

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
    values: HashMap<Vec<u8>, Vec<u8>>,
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl SimulatorState {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn get_value(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.values.get(key)
    }

    pub fn set_value(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.values.insert(key, value);
    }
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[derive(Default)]
pub struct Simulator {
    state: Arc<RwLock<SimulatorState>>,
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl Simulator {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(SimulatorState::new())),
        }
    }

    pub fn with_state(state: Arc<RwLock<SimulatorState>>) -> Self {
        Self { state }
    }

    pub fn get_state(&self) -> Arc<RwLock<SimulatorState>> {
        self.state.clone()
    }

    pub fn execute_wasm(&self, _code: &[u8], method: &str, params: &[u8], _gas: u64) -> Result<Vec<u8>, SimulatorError> {
        // For now, we'll simulate the add function
        if method == "add" {
            let params_str = std::str::from_utf8(params)?;
            let parts: Vec<&str> = params_str.split(',').collect();
            
            if parts.len() != 2 {
                return Err(SimulatorError::ContractExecution(format!("Expected 2 parameters for function '{}'", method)));
            }
            
            let a: i32 = parts[0].trim().parse()?;
            let b: i32 = parts[1].trim().parse()?;
            
            let result = a + b;
            let result_u64 = result as u64;
            
            // Return as uint64 in little-endian format
            Ok(result_u64.to_le_bytes().to_vec())
        } else {
            Err(SimulatorError::ContractExecution(format!("Function '{}' not found in contract", method)))
        }
    }

    pub fn get_balance(&self, account: Address) -> u64 {
        let state = self.state.read().unwrap();
        state.get_value(account.as_bytes())
            .map(|v| {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&v[..8]);
                u64::from_le_bytes(bytes)
            })
            .unwrap_or(0)
    }

    pub fn set_balance(&self, account: Address, balance: u64) {
        let mut state = self.state.write().unwrap();
        state.set_value(account.as_bytes().to_vec(), balance.to_le_bytes().to_vec());
    }

    pub fn create_contract(&self, contract: Address, code: Vec<u8>) {
        let mut state = self.state.write().unwrap();
        state.set_value(contract.as_bytes().to_vec(), code);
    }
}

#[cfg(all(test, feature = "std", not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_creation() {
        let simulator = Simulator::new();
        let state = simulator.get_state();
        assert!(state.read().unwrap().get_value(&[1]).is_none());
    }

    #[test]
    fn test_contract_creation() {
        let simulator = Simulator::new();
        let contract = Address::new(vec![1, 2, 3]);
        let code = vec![4, 5, 6];
        simulator.create_contract(contract.clone(), code.clone());
        
        let state = simulator.get_state();
        assert_eq!(state.read().unwrap().get_value(&contract.as_bytes()).unwrap(), &code);
    }

    #[test]
    fn test_balance() {
        let simulator = Simulator::new();
        let account = Address::new(vec![1, 2, 3]);
        let balance = 100;
        simulator.set_balance(account.clone(), balance);
        assert_eq!(simulator.get_balance(account), balance);
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
