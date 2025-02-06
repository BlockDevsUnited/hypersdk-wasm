// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

extern crate alloc;

use std::sync::Arc;
use parking_lot::RwLock;
use simulator::{Simulator as BaseSimulator, Address as SimAddress};
use crate::types::Address as WasmlAddress;
use displaydoc::Display;
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum ExternalCallError {
    #[displaydoc("Failed to execute: {0}")]
    Execution(String),
}

impl From<WasmlAddress> for SimAddress {
    fn from(addr: WasmlAddress) -> Self {
        SimAddress(addr.as_bytes().to_vec())
    }
}

pub struct Context {
    host_accessor: Arc<RwLock<BaseSimulator>>,
    actor: WasmlAddress,
}

impl Context {
    pub fn new(host_accessor: Arc<RwLock<BaseSimulator>>, actor: WasmlAddress) -> Self {
        Self {
            host_accessor,
            actor,
        }
    }

    pub fn with_actor(actor: WasmlAddress) -> Self {
        Self {
            host_accessor: Arc::new(RwLock::new(BaseSimulator::new(actor.clone().into()))),
            actor,
        }
    }

    pub fn get_balance(&self, account: &WasmlAddress) -> Result<u64, ExternalCallError> {
        let vm = self.host_accessor.read();
        Ok(vm.get_balance(account.clone().into()))
    }

    pub fn set_balance(&self, account: &WasmlAddress, balance: u64) -> Result<(), ExternalCallError> {
        let mut vm = self.host_accessor.write();
        vm.set_balance(account.clone().into(), balance);
        Ok(())
    }

    pub async fn call_contract(&self, contract: &[u8], method: &str, args: &[u8], gas: u64) -> Result<Vec<u8>, ExternalCallError> {
        let mut vm = self.host_accessor.write();
        vm.execute(contract, method, args, gas)
            .await
            .map_err(|e| ExternalCallError::Execution(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockContext;

    impl BaseSimulator for MockContext {
        fn get_balance(&self, account: SimAddress) -> u64 {
            0
        }

        fn set_balance(&mut self, account: SimAddress, balance: u64) {
        }

        async fn execute(&mut self, contract: &[u8], method: &str, args: &[u8], gas: u64) -> Result<Vec<u8>, String> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_context() {
        let mock_context = Arc::new(RwLock::new(MockContext));
        let test_address = WasmlAddress::new([0; 33]); // Create a test address with zeros
        let ctx = Context::new(mock_context, test_address.clone());
        assert!(ctx.get_balance(&test_address).is_ok());
    }
}
