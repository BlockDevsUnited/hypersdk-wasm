// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::error::Error;
use borsh::{BorshDeserialize, BorshSerialize};
use simulator::Simulator as SimVM;
use crate::types::Address;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExternalCallError {
    #[error("Contract execution failed: {0}")]
    ContractExecution(String),
    #[error("Contract creation failed: {0}")]
    ContractCreation(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<Address> for simulator::Address {
    fn from(addr: Address) -> Self {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&addr.as_bytes()[..32]);
        Self::new(bytes)
    }
}

impl From<simulator::Address> for Address {
    fn from(addr: simulator::Address) -> Self {
        let mut bytes = [0u8; 33];
        bytes[..32].copy_from_slice(addr.as_bytes());
        Self::new(bytes)
    }
}

pub struct Simulator {
    vm: SimVM,
    actor: Address,
    height: u64,
    timestamp: u64,
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            vm: SimVM::new(),
            actor: Address::new([0u8; 33]),
            height: 0,
            timestamp: 0,
        }
    }

    pub fn create_contract(&mut self, contract_path: &str) -> Result<CreateContractResult, ExternalCallError> {
        // Read WASM code from file
        let wasm_code = std::fs::read(contract_path)
            .map_err(|e| ExternalCallError::ContractCreation(e.to_string()))?;
        
        // Generate a deterministic address for the contract
        let mut address = [0u8; 33];
        // Use a simple hash of the code for now
        for (i, byte) in wasm_code.iter().enumerate() {
            address[i % 33] ^= byte;
        }
        let contract_addr = Address::new(address);

        self.vm.create_contract(contract_addr.clone().into(), wasm_code);
        
        Ok(CreateContractResult {
            address: contract_addr,
        })
    }

    pub fn call_contract<T: borsh::BorshDeserialize, U: borsh::BorshSerialize>(
        &mut self,
        contract: Address,
        method: &str,
        params: U,
        gas: u64,
    ) -> Result<Vec<u8>, ExternalCallError> {
        let args = borsh::to_vec(&params)
            .map_err(|e| ExternalCallError::ContractExecution(e.to_string()))?;
        
        let result = self.vm.call_contract(contract.into(), method, &args, gas)
            .map_err(|e| ExternalCallError::ContractExecution(e.to_string()))?;
        
        Ok(result)
    }

    pub fn get_actor(&self) -> Address {
        self.actor.clone()
    }

    pub fn set_actor(&mut self, actor: Address) {
        self.actor = actor;
    }

    pub fn get_height(&self) -> u64 {
        self.height
    }

    pub fn set_height(&mut self, height: u64) {
        self.height = height;
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    pub fn get_balance(&self, account: Address) -> u64 {
        self.vm.get_balance(account.into())
    }

    pub fn set_balance(&mut self, account: Address, balance: u64) {
        self.vm.set_balance(account.into(), balance);
    }
}

pub struct CreateContractResult {
    pub address: Address,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_balance_is_zero() {
        let sim = Simulator::new();
        let addr = Address::new([1u8; 33]);
        assert_eq!(sim.get_balance(addr), 0);
    }

    #[test]
    fn get_balance() {
        let mut sim = Simulator::new();
        let addr = Address::new([1u8; 33]);
        let balance = 100;
        sim.set_balance(addr.clone(), balance);
        assert_eq!(sim.get_balance(addr), balance);
    }

    #[test]
    fn set_balance() {
        let mut sim = Simulator::new();
        let addr = Address::new([1u8; 33]);
        let balance = 100;
        sim.set_balance(addr.clone(), balance);
        assert_eq!(sim.get_balance(addr), balance);
    }
}
