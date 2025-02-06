// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::sync::Arc;
use tokio::sync::RwLock;
use simulator::{Simulator as BaseSimulator, Address as SimAddress, SimulatorError};
use crate::types::Address as WasmlAddress;
use thiserror::Error;
use borsh::{BorshSerialize, BorshDeserialize};

#[derive(Debug, Error)]
pub enum ExternalCallError {
    #[error("Failed to execute: {0}")]
    Execution(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<SimulatorError> for ExternalCallError {
    fn from(err: SimulatorError) -> Self {
        ExternalCallError::Execution(err.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Address(pub [u8; 33]);

impl Default for Address {
    fn default() -> Self {
        Self([0; 33])
    }
}

impl Address {
    pub fn new(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 33] {
        &self.0
    }
}

impl From<&WasmlAddress> for Address {
    fn from(addr: &WasmlAddress) -> Self {
        Self(addr.as_bytes().try_into().unwrap())
    }
}

impl From<WasmlAddress> for Address {
    fn from(addr: WasmlAddress) -> Self {
        Self(addr.as_bytes().try_into().unwrap())
    }
}

impl From<&Address> for WasmlAddress {
    fn from(addr: &Address) -> Self {
        WasmlAddress::new(addr.0)
    }
}

impl From<Address> for WasmlAddress {
    fn from(addr: Address) -> Self {
        WasmlAddress::new(addr.0)
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
        Self {
            vm: Arc::new(RwLock::new(BaseSimulator::default())),
            actor: WasmlAddress::new([0u8; 33]),
            height: 0,
            timestamp: 0,
        }
    }
}

impl Simulator {
    pub fn new(actor: WasmlAddress) -> Self {
        Self {
            vm: Arc::new(RwLock::new(BaseSimulator::default())),
            actor,
            height: 0,
            timestamp: 0,
        }
    }

    pub async fn init(&mut self) {
        // No-op for now, can be extended later
    }

    fn to_sim_address(addr: &WasmlAddress) -> SimAddress {
        SimAddress(addr.as_bytes().to_vec())
    }

    pub async fn get_balance(&self, account: WasmlAddress) -> u64 {
        let vm = self.vm.read().await;
        vm.get_balance(Self::to_sim_address(&account))
    }

    pub async fn set_balance(&mut self, account: WasmlAddress, balance: u64) {
        let mut vm = self.vm.write().await;
        vm.set_balance(Self::to_sim_address(&account), balance);
    }

    pub async fn call_contract<U: AsRef<[u8]>>(
        &mut self,
        contract: WasmlAddress,
        method: &str,
        params: U,
        gas: u64,
    ) -> Result<Vec<u8>, ExternalCallError> {
        let mut vm = self.vm.write().await;
        let result = vm.execute(
            contract.as_bytes(),
            method,
            params.as_ref(),
            gas
        ).await;
        result.map_err(|e| ExternalCallError::Execution(e.to_string()))
    }

    pub async fn create_contract(&mut self, wasm_code: Vec<u8>) -> Result<(), ExternalCallError> {
        let mut vm = self.vm.write().await;
        let address = wasm_code.clone();
        vm.create_contract(address, wasm_code)
            .map_err(|e| ExternalCallError::Execution(e.to_string()))
    }

    pub async fn execute(
        &mut self,
        contract: &[u8],
        method: &str,
        args: &[u8],
        gas: u64,
    ) -> Result<Vec<u8>, ExternalCallError> {
        let mut vm = self.vm.write().await;
        let result = vm.execute(contract, method, args, gas).await;
        result.map_err(|e| ExternalCallError::Execution(e.to_string()))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_conversion() {
        let wasml_addr = WasmlAddress::new([1; 33]);
        let sim_addr = Simulator::to_sim_address(&wasml_addr);
        assert_eq!(sim_addr.0, wasml_addr.as_bytes().to_vec());
    }

    #[tokio::test]
    async fn initial_balance_is_zero() {
        let mut sim = Simulator::new(WasmlAddress::new([1u8; 33]));
        sim.init().await;
        let addr = WasmlAddress::new([1u8; 33]);
        assert_eq!(sim.get_balance(addr).await, 0);
    }

    #[tokio::test]
    async fn can_set_balance() {
        let mut sim = Simulator::new(WasmlAddress::new([1u8; 33]));
        sim.init().await;
        let addr = WasmlAddress::new([1u8; 33]);
        let balance = 100;
        sim.set_balance(addr.clone(), balance).await;
        assert_eq!(sim.get_balance(addr).await, balance);
    }

    #[tokio::test]
    async fn test_balance_operations() {
        let mut sim = Simulator::new(WasmlAddress::new([1u8; 33]));
        sim.init().await;
        let addr = WasmlAddress::new([1u8; 33]);
        let balance = 100;
        sim.set_balance(addr.clone(), balance).await;
        assert_eq!(sim.get_balance(addr).await, balance);
    }

    #[tokio::test]
    async fn test_balance_persistence() {
        let mut sim = Simulator::new(WasmlAddress::new([1; 33]));
        sim.init().await;
        let addr = WasmlAddress::new([2; 33]);

        // Initial balance should be 0
        assert_eq!(sim.get_balance(addr).await, 0);

        // Set balance and verify
        let balance = 100;
        sim.set_balance(addr, balance).await;
        assert_eq!(sim.get_balance(addr).await, balance);
    }
}
