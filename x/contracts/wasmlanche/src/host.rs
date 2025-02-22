// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, string::String, vec::Vec};

#[cfg(feature = "std")]
use std::collections::BTreeMap;

use core::future::Future;
use core::pin::Pin;
use spin::RwLock;

use crate::{
    error::Error,
    events::{Event, EventLog},
    gas::GasCounter,
    simulator::Simulator,
    types::WasmlAddress,
};

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
            state: RwLock::new(HostState {
                balances: BTreeMap::new(),
                storage: BTreeMap::new(),
                event_log: EventLog::new(),
                gas_counter: GasCounter::new(1000),
            }),
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

#[async_trait::async_trait]
impl Simulator for HostImpl {
    async fn execute(
        &mut self,
        actor: &WasmlAddress,
        target: &[u8],
        method: &str,
        args: &[u8],
        gas: u64,
    ) -> Result<Vec<u8>, String> {
        // For allocate functions, we need to handle them specially
        if method == "allocate" {
            // For allocate, we expect the input to be the data to allocate
            let size = args.len() as i32;
            Ok((size as i32).to_le_bytes().to_vec())
        } else if method == "allocate_context" {
            // For allocate_context, we expect a 4-byte size parameter
            if args.len() != 4 {
                return Err("allocate_context requires a 4-byte size parameter".to_string());
            }
            let size = i32::from_le_bytes(args.try_into().unwrap());
            Ok((size as i32).to_le_bytes().to_vec())
        } else {
            // For other methods, just return empty for now
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state() {
        let mut host = HostImpl::new(WasmlAddress::default());
        let key = b"test_key";
        let value = b"test_value";

        // Test store and get
        let result = host.store_state(key, value);
        assert!(result.is_ok());

        let result = host.get_state(key);
        assert_eq!(result.unwrap(), Some(value.to_vec()));

        // Test delete
        let result = host.delete_state(key);
        assert_eq!(result.unwrap(), Some(value.to_vec()));

        let result = host.get_state(key);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_balance() {
        let mut host = HostImpl::new(WasmlAddress::default());
        let account = WasmlAddress::default();
        let amount = 100;

        // Test initial balance
        assert_eq!(host.get_balance(&account), 0);

        // Test set balance
        host.set_balance(&account, amount);
        assert_eq!(host.get_balance(&account), amount);
    }

    #[test]
    fn test_gas_charging() {
        let mut host = HostImpl::new(WasmlAddress::default());
        let initial_gas = 1000;
        let charge = 500;

        // Test initial gas
        assert_eq!(host.remaining_gas(), initial_gas);

        // Test charging gas
        let result = host.charge_gas(charge);
        assert!(result.is_ok());
        assert_eq!(host.remaining_gas(), initial_gas - charge);

        // Test out of gas
        let result = host.charge_gas(initial_gas);
        assert!(result.is_err());
    }

    #[test]
    fn test_events() {
        let mut host = HostImpl::new(WasmlAddress::default());
        let event = Event::new("test", "data");

        // Test add event
        let result = host.add_event(event.clone());
        assert!(result.is_ok());

        // Test get events
        assert_eq!(host.get_events(), vec![event.clone()]);

        // Test emit event
        let result = host.emit_event(event.clone());
        assert!(result.is_ok());
        assert_eq!(host.get_events(), vec![event.clone(), event]);
    }
}
