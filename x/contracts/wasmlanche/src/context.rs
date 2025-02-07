// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    error::Error,
    events::Event,
    gas::GasCounter,
    host::Host,
    simulator::Simulator,
    types::WasmlAddress,
    state::{StateAccess, StateKey, Error as StateError},
};

#[derive(Debug)]
pub struct Context {
    actor: WasmlAddress,
    height: u64,
    timestamp: u64,
    host: Arc<RwLock<Host>>,
    gas_counter: Option<GasCounter>,
}

impl Context {
    pub fn new(
        actor: WasmlAddress,
        height: u64,
        timestamp: u64,
        host: Arc<RwLock<Host>>,
        gas_counter: Option<GasCounter>,
    ) -> Self {
        Self {
            actor,
            height,
            timestamp,
            host,
            gas_counter,
        }
    }

    pub fn actor(&self) -> &WasmlAddress {
        &self.actor
    }

    pub async fn get_balance(&self, account: &WasmlAddress) -> Result<u64, Error> {
        let host = self.host.read().await;
        Ok(Simulator::get_balance(&*host, account).await)
    }

    pub async fn transfer(
        &mut self,
        from: &WasmlAddress,
        to: &WasmlAddress,
        amount: u64,
    ) -> Result<(), Error> {
        let mut host = self.host.write().await;
        let from_balance = Simulator::get_balance(&*host, from).await;
        if from_balance < amount {
            return Err(Error::State("Insufficient balance".to_string()));
        }

        Simulator::set_balance(&mut *host, from, from_balance - amount).await;
        let to_balance = Simulator::get_balance(&*host, to).await;
        Simulator::set_balance(&mut *host, to, to_balance + amount).await;
        Ok(())
    }

    pub async fn call_contract(
        &mut self,
        target: &[u8],
        method: &str,
        args: &[u8],
        gas: u64,
    ) -> Result<Vec<u8>, Error> {
        let mut host = self.host.write().await;
        host.execute(&self.actor, target, method, args, gas)
            .await
            .map_err(|e| Error::State(e.to_string()))
    }

    pub async fn get_events(&self) -> Vec<Event> {
        let host = self.host.read().await;
        host.get_events().await.unwrap_or_default()
    }

    pub async fn add_event(&mut self, event: Event) -> Result<(), Error> {
        let mut host = self.host.write().await;
        host.add_event(event).await.map_err(|e| Error::Event(e.to_string()))
    }
}

#[async_trait::async_trait]
impl StateAccess for Context {
    async fn store_state<S: borsh::BorshSerialize + StateKey + Send + Sync>(
        &mut self,
        state: &S,
    ) -> Result<(), StateError> {
        let mut host = self.host.write().await;
        let key = S::get_key();
        host.store_state(&key, &state.try_to_vec()?)
            .await
            .map_err(|e| StateError::StateError(e.to_string()))
    }

    async fn get_state<S: borsh::BorshDeserialize + StateKey + Send + Sync>(
        &self,
    ) -> Result<Option<S>, StateError> {
        let host = self.host.read().await;
        let key = S::get_key();
        match host.get_state(&key).await {
            Ok(Some(bytes)) => Ok(Some(borsh::BorshDeserialize::try_from_slice(&bytes)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::StateError(e.to_string())),
        }
    }

    async fn delete_state<S: borsh::BorshDeserialize + StateKey + Send + Sync>(
        &mut self,
    ) -> Result<Option<S>, StateError> {
        let mut host = self.host.write().await;
        let key = S::get_key();
        match host.delete_state(&key).await {
            Ok(Some(bytes)) => Ok(Some(borsh::BorshDeserialize::try_from_slice(&bytes)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::StateError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::HostState;

    #[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
    struct TestState {
        value: String,
    }

    impl StateKey for TestState {
        fn get_key() -> Vec<u8> {
            b"test_state".to_vec()
        }
    }

    #[tokio::test]
    async fn test_state_operations() {
        let mut context = Context::new(
            WasmlAddress::new(vec![1, 2, 3]),
            0,
            0,
            Arc::new(RwLock::new(Host::new(Arc::new(RwLock::new(HostState::default()))))),
            None,
        );
        let test_state = TestState {
            value: "test".to_string(),
        };

        // Test store_state
        context.store_state(&test_state).await.unwrap();

        // Test get_state
        let retrieved: Option<TestState> = context.get_state().await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "test");

        // Test delete_state
        let deleted: Option<TestState> = context.delete_state().await.unwrap();
        assert!(deleted.is_some());
        assert_eq!(deleted.unwrap().value, "test");

        // Verify state is deleted
        let retrieved: Option<TestState> = context.get_state().await.unwrap();
        assert!(retrieved.is_none());
    }
}
