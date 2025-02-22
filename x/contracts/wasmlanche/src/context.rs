// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::sync::Arc;
use std::string::ToString;
#[cfg(not(feature = "std"))]
use alloc::string::String;
use borsh::maybestd::string::ToString as BorshToString;
use tokio::sync::RwLock;
use crate::{
    error::Error,
    events::Event,
    gas::GasCounter,
    host::{Host, HostState},
    simulator::Simulator,
    types::{Address, Gas, WasmlAddress, ContractId},
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
            return Err(Error::State("Insufficient balance"));
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
            .map_err(|_| Error::State("Failed to execute contract"))
    }

    pub async fn get_events(&self) -> Vec<Event> {
        let host = self.host.read().await;
        host.get_events().await.unwrap_or_default()
    }

    pub async fn add_event(&mut self, event: Event) -> Result<(), Error> {
        let mut host = self.host.write().await;
        host.add_event(event).await.map_err(|_| Error::Event("Failed to add event"))
    }

    pub async fn send(&mut self, recipient: &[u8], amount: u64) -> Result<(), Error> {
        let recipient_addr = WasmlAddress::from(recipient);
        let actor = self.actor.clone();
        self.transfer(&actor, &recipient_addr, amount).await
    }

    pub async fn deploy(&mut self, contract_id: ContractId) -> Result<WasmlAddress, Error> {
        let address = WasmlAddress::from(contract_id.as_bytes().as_ref());
        let mut host = self.host.write().await;
        let key = format!("contract:{}", hex::encode(address.as_bytes()));
        host.store_state(key.as_bytes(), &[]).await?;
        Ok(address)
    }
}

#[async_trait::async_trait]
impl StateAccess for Context {
    async fn store_state<S: borsh::BorshSerialize + StateKey + Send + Sync>(&mut self, state: &S) -> Result<(), StateError> {
        let key = S::get_key();
        let bytes = state.try_to_vec()
            .map_err(|e| StateError::Serialization(e.to_string()))?;
        let mut host = self.host.write().await;
        host.store_state(&key, &bytes)
            .await
            .map_err(|e| StateError::State(e.to_string()))
    }

    async fn get_state<S: borsh::BorshDeserialize + StateKey + Send + Sync>(&self) -> Result<Option<S>, StateError> {
        let key = S::get_key();
        let host = self.host.read().await;
        match host.get_state(&key).await {
            Ok(Some(bytes)) => {
                S::try_from_slice(&bytes)
                    .map(Some)
                    .map_err(|e| StateError::Serialization(e.to_string()))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::State(e.to_string())),
        }
    }

    async fn delete_state<S: borsh::BorshDeserialize + StateKey + Send + Sync>(&mut self) -> Result<Option<S>, StateError> {
        let key = S::get_key();
        let mut host = self.host.write().await;
        match host.delete_state(&key).await {
            Ok(Some(bytes)) => {
                S::try_from_slice(&bytes)
                    .map(Some)
                    .map_err(|e| StateError::Serialization(e.to_string()))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::State(e.to_string())),
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
        let retrieved: Option<TestState> = context.get_state::<TestState>().await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "test");

        // Test delete_state
        let deleted: Option<TestState> = context.delete_state::<TestState>().await.unwrap();
        assert!(deleted.is_some());
        assert_eq!(deleted.unwrap().value, "test");

        // Verify state is deleted
        let retrieved: Option<TestState> = context.get_state::<TestState>().await.unwrap();
        assert!(retrieved.is_none());
    }
}
