// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::error::Error;
use thiserror::Error;

pub mod state;
use state::SimpleState;

#[derive(Debug, Error)]
pub enum SimulatorError {
    #[error("Contract creation failed: {0}")]
    ContractCreation(String),
    #[error("Contract execution failed: {0}")]
    ContractExecution(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct Address([u8; 32]);

impl Address {
    pub fn new(bytes: [u8; 32]) -> Self {
        Address(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Default for Address {
    fn default() -> Self {
        Address([0u8; 32])
    }
}

#[derive(Debug)]
pub struct CreateContractResult {
    pub address: Address,
}

pub struct Simulator {
    state: SimpleState,
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            state: SimpleState::new(),
        }
    }

    pub fn create_contract(&mut self, contract: Address, code: Vec<u8>) {
        self.state.insert(
            contract.as_bytes().to_vec().into_boxed_slice(),
            code.into_boxed_slice(),
        );
    }

    pub fn get_contract_code(&self, contract: &Address) -> Option<Vec<u8>> {
        self.state.get_value(contract.as_bytes())
            .map(|code| code.to_vec())
    }

    pub fn call_contract(
        &mut self,
        contract: Address,
        method: &str,
        args: &[u8],
        gas: u64,
    ) -> Result<Vec<u8>, SimulatorError> {
        // Get contract code from state
        let code = self.state.get_value(contract.as_bytes())
            .ok_or_else(|| SimulatorError::ContractExecution("Contract not found".into()))?;

        // TODO: Implement WASM execution
        // For now, just return the input args
        Ok(args.to_vec())
    }

    pub fn get_balance(&self, account: Address) -> u64 {
        // Get balance from state, default to 0
        self.state.get_value(account.as_bytes())
            .and_then(|bytes| bytes.try_into().ok())
            .map(u64::from_be_bytes)
            .unwrap_or(0)
    }

    pub fn set_balance(&mut self, account: Address, balance: u64) {
        self.state.insert(
            account.as_bytes().to_vec().into_boxed_slice(),
            balance.to_be_bytes().to_vec().into_boxed_slice(),
        );
    }

    pub fn create_contract_with_code(&mut self, contract: Address, code: Vec<u8>) {
        self.state.insert(
            contract.as_bytes().to_vec().into_boxed_slice(),
            code.into_boxed_slice(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_creation() {
        let simulator = Simulator::new();
        assert!(simulator.get_contract_code(&Address::default()).is_none());
    }

    #[test]
    fn test_contract_creation() {
        let mut simulator = Simulator::new();
        let contract = Address::new([1u8; 32]);
        let code = vec![1, 2, 3, 4];
        
        simulator.create_contract(contract.clone(), code.clone());
        assert_eq!(simulator.get_contract_code(&contract), Some(code));
    }

    #[test]
    fn test_state_management() {
        let mut simulator = Simulator::new();
        let addr = Address::new([2u8; 32]);
        let value = vec![5, 6, 7, 8];
        
        simulator.create_contract(addr.clone(), value.clone());
        assert_eq!(simulator.get_contract_code(&addr), Some(value));
    }
}
