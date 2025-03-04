// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, string::String, vec::Vec};

#[cfg(feature = "std")]
use std::collections::BTreeMap;

#[cfg(all(feature = "simulator", feature = "std", not(target_arch = "wasm32")))]
use core::future::Future;
#[cfg(all(feature = "simulator", feature = "std", not(target_arch = "wasm32")))]
use core::pin::Pin;
use spin::RwLock;

use crate::{
    error::Error,
    events::{Event, EventLog},
    gas::GasCounter,
    types::WasmlAddress,
};

#[cfg(all(feature = "simulator", feature = "std", not(target_arch = "wasm32")))]
use crate::simulator::{Simulator, SimulatorExt};

#[cfg(all(feature = "simulator", target_arch = "wasm32"))]
use crate::simulator::Simulator;

/// Host state for a contract
#[derive(Default)]
pub struct HostState {
    pub(crate) balances: BTreeMap<Vec<u8>, u64>,
    pub(crate) storage: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(crate) event_log: EventLog,
    pub gas_counter: GasCounter,
}

impl HostState {
    pub fn store(&mut self, key: &[u8], value: Vec<u8>) {
        self.storage.insert(key.to_vec(), value);
    }

    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.storage.get(key)
    }

    pub fn get_events(&self) -> Vec<Event> {
        self.event_log.events().iter().cloned().collect()
    }

    pub fn delete(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.storage.remove(key)
    }
}

/// Interface for host functionality
pub trait Host: Send + Sync {
    fn store_state(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error>;
    fn get_state(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error>;
    fn delete_state(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, Error>;
    fn get_events(&self) -> Vec<Event>;
    fn add_event(&mut self, event: Event) -> Result<(), Error>;
    fn charge_gas(&mut self, amount: u64) -> Result<(), Error>;
    fn remaining_gas(&self) -> u64;
    fn get_balance(&self, account: &WasmlAddress) -> u64;
    fn set_balance(&mut self, account: &WasmlAddress, amount: u64);
    fn emit_event(&mut self, event: Event) -> Result<(), Error>;
}

/// Default implementation of Host
pub struct HostImpl {
    state: RwLock<HostState>,
}

impl HostImpl {
    pub fn new(_actor: WasmlAddress) -> Self {
        Self {
            state: RwLock::new(HostState::default()),
        }
    }
}

impl Host for HostImpl {
    fn store_state(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        let mut state = self.state.write();
        state.store(key, value.to_vec());
        Ok(())
    }

    fn get_state(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let state = self.state.read();
        Ok(state.get(key).map(|v| v.clone()))
    }

    fn delete_state(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let mut state = self.state.write();
        Ok(state.delete(key))
    }

    fn get_events(&self) -> Vec<Event> {
        let state = self.state.read();
        state.get_events()
    }

    fn add_event(&mut self, event: Event) -> Result<(), Error> {
        let mut state = self.state.write();
        state.event_log.add_event(event)
    }

    fn charge_gas(&mut self, amount: u64) -> Result<(), Error> {
        let mut state = self.state.write();
        state.gas_counter.charge_gas(amount)
    }

    fn remaining_gas(&self) -> u64 {
        let state = self.state.read();
        state.gas_counter.gas_remaining()
    }

    fn get_balance(&self, account: &WasmlAddress) -> u64 {
        let state = self.state.read();
        state.balances.get(&account.as_bytes().to_vec()).copied().unwrap_or(0)
    }

    fn set_balance(&mut self, account: &WasmlAddress, amount: u64) {
        let mut state = self.state.write();
        state.balances.insert(account.as_bytes().to_vec(), amount);
    }

    fn emit_event(&mut self, event: Event) -> Result<(), Error> {
        self.add_event(event)
    }
}

#[cfg(all(feature = "simulator", feature = "std", not(target_arch = "wasm32")))]
#[async_trait::async_trait]
impl SimulatorExt for HostImpl {
    fn get_balance_async<'a>(&'a self, _actor: &'a WasmlAddress) -> Pin<Box<dyn Future<Output = u64> + Send + 'a>> {
        Box::pin(async move { Simulator::get_balance(self, _actor) })
    }

    fn set_balance_async<'a>(&'a mut self, _actor: &'a WasmlAddress, balance: u64) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move { Simulator::set_balance(self, _actor, balance) })
    }

    fn store_state<'a>(&'a mut self, key: &'a [u8], value: &'a [u8]) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            Host::store_state(self, key, value).unwrap();
        })
    }

    fn get_state<'a>(&'a self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>> {
        Box::pin(async move {
            Host::get_state(self, key).unwrap()
        })
    }

    fn delete_state<'a>(&'a mut self, key: &'a [u8]) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'a>> {
        Box::pin(async move {
            Host::delete_state(self, key).unwrap()
        })
    }

    fn execute<'a>(
        &'a mut self,
        _actor: &'a WasmlAddress,
        _target: &'a [u8],
        _method: &'a str,
        _args: &'a [u8],
        gas: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send + 'a>> {
        Box::pin(async move {
            let mut state = self.state.write();
            state.gas_counter = GasCounter::new(gas);
            
            // For now, just return empty result
            // TODO: Implement actual WASM execution
            Ok(vec![])
        })
    }

    fn remaining_fuel_async(&self) -> u64 {
        Simulator::remaining_fuel(self)
    }

    fn get_events_async(&self) -> Vec<Event> {
        Simulator::get_events(self)
    }
}

#[cfg(feature = "simulator")]
impl Simulator for HostImpl {
    fn get_balance(&self, account: &WasmlAddress) -> u64 {
        Host::get_balance(self, account)
    }

    fn set_balance(&mut self, account: &WasmlAddress, balance: u64) {
        Host::set_balance(self, account, balance)
    }

    fn remaining_fuel(&self) -> u64 {
        Host::remaining_gas(self)
    }

    fn get_events(&self) -> Vec<Event> {
        Host::get_events(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state() {
        let mut host = HostImpl::new(WasmlAddress::new([1; 32]));

        Host::store_state(&mut host, b"key", b"value").unwrap();

        let value = Host::get_state(&host, b"key").unwrap();
        assert_eq!(value, Some(b"value".to_vec()));

        let deleted = Host::delete_state(&mut host, b"key").unwrap();
        assert_eq!(deleted, Some(b"value".to_vec()));

        let value = Host::get_state(&host, b"key").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_balance() {
        let mut host = HostImpl::new(WasmlAddress::new([1; 32]));
        let account = WasmlAddress::new([2; 32]);

        assert_eq!(Host::get_balance(&host, &account), 0);

        Host::set_balance(&mut host, &account, 100);
        assert_eq!(Host::get_balance(&host, &account), 100);
    }

    #[test]
    fn test_gas_charging() {
        let mut host = HostImpl::new(WasmlAddress::new([1; 32]));

        // Test charging gas
        host.charge_gas(100).unwrap();
        assert_eq!(host.remaining_gas(), 999900);

        // Test charging more than remaining
        assert!(host.charge_gas(1000000).is_err());
    }
}
