// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::sync::Arc;
use tokio::sync::RwLock;
use simulator::Simulator as BaseSimulator;
use crate::types::Address as WasmlAddress;
use thiserror::Error;
use borsh::BorshSerialize;

#[derive(Debug, Error)]
pub enum ExternalCallError {
    #[error("Contract execution failed: {0}")]
    ContractExecution(String),
    #[error("Contract creation failed: {0}")]
    ContractCreation(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<WasmlAddress> for simulator::Address {
    fn from(addr: WasmlAddress) -> Self {
        simulator::Address::new(addr.as_bytes().to_vec())
    }
}

impl From<simulator::Address> for WasmlAddress {
    fn from(addr: simulator::Address) -> Self {
        let mut bytes = [0u8; 33];
        bytes[..32].copy_from_slice(addr.as_bytes());
        WasmlAddress::new(bytes)
    }
}

pub struct Simulator {
    vm: Arc<RwLock<BaseSimulator>>,
    actor: WasmlAddress,
    height: u64,
    timestamp: u64,
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            vm: Arc::new(RwLock::new(BaseSimulator::new())),
            actor: WasmlAddress::new([0u8; 33]),
            height: 0,
            timestamp: 0,
        }
    }

    pub async fn create_contract(&mut self, wasm_code: Vec<u8>) -> Result<CreateContractResult, ExternalCallError> {
        // Create a deterministic address from the wasm code
        let mut address = [0u8; 33];
        for (i, byte) in wasm_code.iter().enumerate() {
            address[i % 33] ^= byte;
        }
        let contract_addr = WasmlAddress::new(address);

        self.vm.write().await.create_contract(contract_addr.clone().into(), wasm_code).await;
        
        Ok(CreateContractResult {
            address: contract_addr,
        })
    }

    pub async fn call_contract<U: BorshSerialize>(
        &mut self,
        contract: WasmlAddress,
        method: &str,
        params: U,
        gas: u64,
    ) -> Result<Vec<u8>, ExternalCallError> {
        // Get contract code
        let vm = self.vm.read().await;
        let state = vm.get_state();
        let state_guard = state.read().await;
        let contract_code = state_guard.get_value(contract.as_bytes()).await
            .ok_or_else(|| ExternalCallError::ContractExecution("Contract not found".to_string()))?;
        drop(state_guard);
        drop(vm);
        
        // Serialize parameters
        let args = borsh::to_vec(&params)
            .map_err(|e| ExternalCallError::ContractExecution(e.to_string()))?;
        
        // Execute contract
        let mut vm = self.vm.write().await;
        let result = vm.execute(&contract_code, method, &args, gas)
            .await
            .map_err(|e| ExternalCallError::ContractExecution(e.to_string()))?;
        
        Ok(result)
    }

    pub async fn get_actor(&self) -> WasmlAddress {
        self.actor.clone()
    }

    pub async fn set_actor(&mut self, actor: WasmlAddress) {
        self.actor = actor;
    }

    pub async fn get_height(&self) -> u64 {
        self.height
    }

    pub async fn set_height(&mut self, height: u64) {
        self.height = height;
    }

    pub async fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub async fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    pub async fn get_balance(&self, account: WasmlAddress) -> u64 {
        let vm = self.vm.read().await;
        vm.get_balance(account.into()).await
    }

    pub async fn set_balance(&mut self, account: WasmlAddress, balance: u64) {
        self.vm.write().await.set_balance(account.into(), balance).await;
    }

    pub async fn execute(&mut self, contract: &[u8], method: &str, args: &[u8], gas: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut vm = self.vm.write().await;
        vm.execute(contract, method, args, gas)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

pub struct CreateContractResult {
    pub address: WasmlAddress,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn initial_balance_is_zero() {
        let sim = Simulator::new();
        let addr = WasmlAddress::new([1u8; 33]);
        assert_eq!(sim.get_balance(addr).await, 0);
    }

    #[tokio::test]
    async fn get_balance() {
        let mut sim = Simulator::new();
        let addr = WasmlAddress::new([1u8; 33]);
        let balance = 100;
        sim.set_balance(addr.clone(), balance).await;
        assert_eq!(sim.get_balance(addr).await, balance);
    }

    #[tokio::test]
    async fn set_balance() {
        let mut sim = Simulator::new();
        let addr = WasmlAddress::new([1u8; 33]);
        let balance = 100;
        sim.set_balance(addr.clone(), balance).await;
        assert_eq!(sim.get_balance(addr).await, balance);
    }
}
