// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::sync::Arc;
use tokio::sync::RwLock;
use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Sha256, Digest};
use sha3::Keccak256;
use ed25519_dalek::{PublicKey, Signature, Verifier};
use async_trait::async_trait;
use std::{
    future::Future,
    pin::Pin,
};

use crate::{
    error::Error,
    events::{Event, EventLog},
    gas::GasCounter,
    simulator::Simulator,
    state::StateAccess,
    types::WasmlAddress,
};

#[derive(Debug, Default)]
pub struct HostState {
    pub event_log: EventLog,
    pub gas_counter: GasCounter,
    balances: std::collections::HashMap<Vec<u8>, u64>,
}

pub trait SimulatorWithDebug: Simulator + std::fmt::Debug {}
impl<T: Simulator + std::fmt::Debug> SimulatorWithDebug for T {}

#[derive(Debug)]
pub struct Host {
    state: Arc<RwLock<HostState>>,
}

impl Host {
    pub fn new(state: Arc<RwLock<HostState>>) -> Self {
        Self { state }
    }

    pub async fn add_event(&mut self, event: Event) -> Result<(), Error> {
        let mut state = self.state.write().await;
        state.event_log.add_event(event)
    }

    pub async fn charge_gas(&mut self, amount: u64) -> Result<(), Error> {
        let mut state = self.state.write().await;
        state.gas_counter.charge_gas(amount)?;
        Ok(())
    }

    pub async fn get_state(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let state = self.state.read().await;
        Ok(state.event_log.get_state(key).cloned())
    }

    pub async fn store_state(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        let mut state = self.state.write().await;
        state.event_log.store_state(key, value).map_err(|e| Error::Event(e.to_string()))
    }

    pub async fn delete_state(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let mut state = self.state.write().await;
        let existing = state.event_log.get_state(key).cloned();
        if existing.is_some() {
            state.event_log.delete_state(key).map_err(|e| Error::Event(e.to_string()))?;
        }
        Ok(existing)
    }

    pub async fn execute(
        &mut self,
        _actor: &WasmlAddress,
        _target: &[u8],
        _method: &str,
        _args: &[u8],
        gas: u64,
    ) -> Result<Vec<u8>, Error> {
        self.charge_gas(gas).await?;
        Ok(Vec::new())
    }

    pub async fn get_events(&self) -> Result<Vec<Event>, Error> {
        let state = self.state.read().await;
        Ok(state.event_log.events().iter().cloned().collect())
    }

    pub fn get_events_blocking(&self) -> Vec<Event> {
        let state = self.state.blocking_read();
        state.event_log.events().iter().cloned().collect()
    }

    pub fn get_contract_events(&self) -> Vec<Event> {
        let state = self.state.blocking_read();
        state.event_log.events().iter().cloned().collect()
    }

    pub fn get_all_events(&self) -> Vec<Event> {
        futures::executor::block_on(async {
            let state = self.state.read().await;
            state.event_log.events().iter().cloned().collect()
        })
    }

    pub fn get_events_for_contract(&self) -> Vec<Event> {
        futures::executor::block_on(async {
            let state = self.state.read().await;
            state.event_log.events().iter().cloned().collect::<Vec<_>>()
        })
    }

    pub fn get_events_for_contract_blocking(&self) -> Vec<Event> {
        futures::executor::block_on(async {
            let state = self.state.read().await;
            state.event_log.events().iter().cloned().collect::<Vec<_>>()
        })
    }

    pub fn get_events_for_contract_async(&self) -> Vec<Event> {
        futures::executor::block_on(async {
            let state = self.state.read().await;
            state.event_log.events().iter().cloned().collect::<Vec<_>>()
        })
    }

    pub fn remaining_gas(&self) -> Option<u64> {
        futures::executor::block_on(async {
            let state = self.state.read().await;
            Some(state.gas_counter.gas_remaining())
        })
    }

    pub fn sha256(&self, data: &[u8]) -> Result<[u8; 32], Error> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Ok(hasher.finalize().into())
    }

    pub fn keccak256(&self, data: &[u8]) -> Result<[u8; 32], Error> {
        let mut hasher = Keccak256::new();
        hasher.update(data);
        Ok(hasher.finalize().into())
    }

    pub fn ed25519_verify(
        &self,
        pubkey: &[u8],
        msg: &[u8],
        sig: &[u8],
    ) -> Result<bool, Error> {
        let public_key = PublicKey::from_bytes(pubkey)
            .map_err(|e| Error::Crypto(e.to_string()))?;
        let signature = Signature::from_bytes(sig)
            .map_err(|e| Error::Crypto(e.to_string()))?;
        Ok(public_key.verify(msg, &signature).is_ok())
    }
}

#[async_trait]
impl Simulator for Host {
    fn get_balance<'a>(&'a self, account: &'a WasmlAddress) -> Pin<Box<dyn Future<Output = u64> + Send + 'a>> {
        Box::pin(async move {
            let state = self.state.read().await;
            state.balances.get(&account.as_bytes().to_vec()).copied().unwrap_or(0)
        })
    }

    fn set_balance<'a>(&'a mut self, account: &'a WasmlAddress, balance: u64) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let mut state = self.state.write().await;
            state.balances.insert(account.as_bytes().to_vec(), balance);
        })
    }

    fn remaining_fuel(&self) -> u64 {
        self.remaining_gas().unwrap_or(0)
    }

    fn get_events(&self) -> Vec<Event> {
        self.state.blocking_read().event_log.events().iter().cloned().collect()
    }

    fn store_state<'a>(&'a mut self, key: &'a [u8], value: &'a [u8]) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            self.store_state(key, value).await.unwrap_or(());
        })
    }

    fn get_state<'a>(&'a self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>> {
        Box::pin(async move {
            self.get_state(key).await.unwrap_or(None)
        })
    }

    fn delete_state<'a>(&'a mut self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>> {
        Box::pin(async move {
            self.delete_state(key).await.unwrap_or(None)
        })
    }

    fn execute<'a>(
        &'a mut self,
        actor: &'a WasmlAddress,
        target: &'a [u8],
        method: &'a str,
        args: &'a [u8],
        gas: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.execute(actor, target, method, args, gas)
                .await
                .map_err(|e| e.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_host_state() {
        let state = Arc::new(RwLock::new(HostState::default()));
        let mut host = Host::new(state);

        // Test store_state
        host.store_state(b"key", b"value").await.unwrap();

        // Test get_state
        let value = host.get_state(b"key").await.unwrap();
        assert_eq!(value, Some(b"value".to_vec()));

        // Test delete_state
        let deleted = host.delete_state(b"key").await.unwrap();
        assert_eq!(deleted, Some(b"value".to_vec()));

        // Verify state is deleted
        let value = host.get_state(b"key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_gas_charging() {
        let state = Arc::new(RwLock::new(HostState::default()));
        let mut host = Host::new(state);

        // Test charging gas
        host.charge_gas(100).await.unwrap();
        assert_eq!(host.remaining_gas(), Some(999900));

        // Test charging more than remaining
        assert!(host.charge_gas(1000000).await.is_err());
    }

    #[tokio::test]
    async fn test_balance_operations() {
        let state = Arc::new(RwLock::new(HostState::default()));
        let mut host = Host::new(state);
        let account = WasmlAddress::new(vec![1, 2, 3]);

        // Test initial balance
        assert_eq!(host.get_balance(&account).await, 0);

        // Test setting balance
        host.set_balance(&account, 100).await;
        assert_eq!(host.get_balance(&account).await, 100);
    }
}
