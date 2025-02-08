// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::{
    pin::Pin,
    future::Future,
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{
    error::Error,
    events::Event,
    gas::GasCounter,
    host::Host,
    simulator::Simulator,
    state::{StateAccess, StateKey, Error as StateError},
    types::WasmlAddress,
    safety::SafetyManager,
};

#[derive(Debug)]
pub struct Context {
    actor: WasmlAddress,
    height: u64,
    timestamp: u64,
    host: Arc<RwLock<Host>>,
    gas_counter: Option<GasCounter>,
    safety_manager: SafetyManager,
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
            safety_manager: SafetyManager::new(),
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
        nonce: Option<u64>,
    ) -> Result<(), Error> {
        // Verify and increment nonce for the sender
        let nonce = nonce.unwrap_or(0);
        if let Err(e) = self.safety_manager.verify_and_increment_nonce(from.as_ref(), nonce) {
            return Err(e);
        }
        
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
        // Check call depth before proceeding
        self.safety_manager.enter_call()?;
        
        let result = {
            let mut host = self.host.write().await;
            match Simulator::execute(&mut *host, &self.actor, target, method, args, gas).await {
                Ok(result) => Ok(result),
                Err(e) => Err(Error::State(e)),
            }
        };
        
        // Always exit the call, even if there was an error
        self.safety_manager.exit_call();
        
        result
    }

    pub async fn get_events(&self) -> Vec<Event> {
        let host = self.host.read().await;
        host.get_events().await.unwrap_or_default()
    }

    pub async fn add_event(&mut self, event: Event) -> Result<(), Error> {
        let mut host = self.host.write().await;
        host.add_event(event).await.map_err(|e| Error::Event(e.to_string()))
    }

    // Add new methods for nonce management
    pub fn get_nonce(&self, actor: &WasmlAddress) -> u64 {
        self.safety_manager.get_nonce(actor.as_ref())
    }

    // Add method to check protocol version
    pub fn check_protocol_version(&self, version: u32) -> Result<(), Error> {
        self.safety_manager.check_protocol_version(version)
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
            Ok(Some(data)) => Ok(Some(S::try_from_slice(&data)?)),
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
            Ok(Some(data)) => Ok(Some(S::try_from_slice(&data)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::StateError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::host::HostState;

    #[derive(Debug, PartialEq, borsh::BorshSerialize, borsh::BorshDeserialize)]
    struct TestState {
        value: u32,
    }

    impl StateKey for TestState {
        fn get_key() -> Vec<u8> {
            b"test_state".to_vec()
        }
    }

    #[tokio::test]
    async fn test_state_operations() {
        let host_state = Arc::new(RwLock::new(HostState::default()));
        let host = Arc::new(RwLock::new(Host::new(host_state)));
        let mut context = Context::new(
            WasmlAddress::new(vec![1; 32]),
            1,
            1000,
            host,
            None,
        );

        // Test state operations
        let state = TestState { value: 42 };
        context.store_state(&state).await.unwrap();

        let retrieved: Option<TestState> = context.get_state::<TestState>().await.unwrap();
        assert_eq!(retrieved.unwrap().value, 42);

        let deleted: Option<TestState> = context.delete_state::<TestState>().await.unwrap();
        assert_eq!(deleted.unwrap().value, 42);

        let empty: Option<TestState> = context.get_state::<TestState>().await.unwrap();
        assert!(empty.is_none());
    }

    #[tokio::test]
    async fn test_safety_features() {
        let host_state = Arc::new(RwLock::new(HostState::default()));
        let host = Arc::new(RwLock::new(Host::new(host_state)));
        let mut context = Context::new(
            WasmlAddress::new(vec![1; 32]),
            1,
            1000,
            host.clone(),
            None,
        );

        // Test nonce verification
        let actor = WasmlAddress::new(vec![2; 32]);
        assert_eq!(context.get_nonce(&actor), 0);

        // Initialize balance
        {
            let mut host = host.write().await;
            Simulator::set_balance(&mut *host, &actor, 1000).await;
        }

        // Test transfer with nonce 0
        let result = context.transfer(
            &actor,
            &WasmlAddress::new(vec![3; 32]),
            100,
            Some(0),
        ).await;
        assert!(result.is_ok());
        assert_eq!(context.get_nonce(&actor), 1);

        // Test transfer with wrong nonce (should be 1, but we use 0)
        let result = context.transfer(
            &actor,
            &WasmlAddress::new(vec![3; 32]),
            100,
            Some(0),
        ).await;
        assert!(result.is_err());
        if let Err(Error::InvalidNonce(_)) = result {
            // Expected error
        } else {
            panic!("Expected InvalidNonce error");
        }

        // Test call depth by making nested calls up to MAX_CALL_DEPTH
        // We'll use the safety_manager's call depth directly to simulate nested calls
        for _ in 0..8 {
            context.safety_manager.enter_call().unwrap();
        }

        // Test exceeding max call depth
        let result = context.safety_manager.enter_call();
        assert!(result.is_err());
        if let Err(Error::MaxDepthExceeded(_)) = result {
            // Expected error
        } else {
            panic!("Expected MaxDepthExceeded error");
        }

        // Reset call depth
        for _ in 0..8 {
            context.safety_manager.exit_call();
        }

        // Test protocol version
        assert!(context.check_protocol_version(1).is_ok());
        assert!(context.check_protocol_version(2).is_err());
    }
}
