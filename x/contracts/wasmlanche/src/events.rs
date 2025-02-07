#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::vec;

use borsh::{BorshSerialize, BorshDeserialize};
use crate::error::EventError;
use crate::gas::{MAX_EVENT_NAME_LENGTH, MAX_EVENT_DATA_SIZE, MAX_EVENTS_PER_CONTRACT};
use crate::types::WasmlAddress;
use std::collections::{VecDeque, HashMap};
use async_trait::async_trait;

use crate::{
    error::Error,
    state::{StateAccess, StateKey, Error as StateError},
};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum Event {
    StateChange {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Custom {
        contract_addr: WasmlAddress,
        name: String,
        data: Vec<u8>,
        height: u64,
        timestamp: u64,
    }
}

#[derive(Debug, Default)]
pub struct EventLog {
    events: VecDeque<Event>,
    state: HashMap<Vec<u8>, Vec<u8>>,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            state: HashMap::new(),
        }
    }

    pub fn add_event(&mut self, event: Event) -> Result<(), EventError> {
        match &event {
            Event::StateChange { key, value } => {
                if key.len() + value.len() > MAX_EVENT_DATA_SIZE {
                    return Err(Error::DataTooLarge(format!(
                        "State change data must be at most {} bytes",
                        MAX_EVENT_DATA_SIZE
                    )));
                }
                self.state.insert(key.clone(), value.clone());
            }
            Event::Custom { name, data, .. } => {
                if name.len() > MAX_EVENT_NAME_LENGTH {
                    return Err(Error::NameTooLong(format!(
                        "Event name must be at most {} bytes",
                        MAX_EVENT_NAME_LENGTH
                    )));
                }
                if data.len() > MAX_EVENT_DATA_SIZE {
                    return Err(Error::DataTooLarge(format!(
                        "Event data must be at most {} bytes",
                        MAX_EVENT_DATA_SIZE
                    )));
                }
            }
        }
        
        if self.events.len() >= MAX_EVENTS_PER_CONTRACT {
            return Err(Error::TooManyEvents(format!(
                "Contract can emit at most {} events",
                MAX_EVENTS_PER_CONTRACT
            )));
        }
        self.events.push_back(event);
        Ok(())
    }

    pub fn events(&self) -> &VecDeque<Event> {
        &self.events
    }

    pub fn store_state(&mut self, key: &[u8], value: &[u8]) -> Result<(), EventError> {
        self.state.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    pub fn get_state(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.state.get(key)
    }

    pub fn delete_state(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, EventError> {
        Ok(self.state.remove(key))
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[async_trait]
impl StateAccess for EventLog {
    async fn store_state<S: BorshSerialize + StateKey + Send + Sync>(
        &mut self,
        state: &S,
    ) -> Result<(), StateError> {
        let bytes = state.try_to_vec().map_err(|e| StateError::SerializationError(e.to_string()))?;
        let key = S::get_key();
        self.store_state(&key, &bytes)
            .map_err(|e| StateError::StateError(e.to_string()))
    }

    async fn get_state<S: BorshDeserialize + StateKey + Send + Sync>(
        &self,
    ) -> Result<Option<S>, StateError> {
        let key = S::get_key();
        match self.get_state(&key) {
            Some(value) => Ok(Some(S::try_from_slice(value)
                .map_err(|e| StateError::SerializationError(e.to_string()))?)),
            None => Ok(None),
        }
    }

    async fn delete_state<S: BorshDeserialize + StateKey + Send + Sync>(
        &mut self,
    ) -> Result<Option<S>, StateError> {
        let key = S::get_key();
        match self.delete_state(&key) {
            Ok(Some(value)) => Ok(Some(S::try_from_slice(&value)
                .map_err(|e| StateError::SerializationError(e.to_string()))?)),
            Ok(None) => Ok(None),
            Err(e) => Err(StateError::StateError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(BorshSerialize, BorshDeserialize)]
    struct TestState {
        value: String,
    }

    impl StateKey for TestState {
        fn get_key() -> Vec<u8> {
            b"test_state".to_vec()
        }
    }

    #[tokio::test]
    async fn test_event_log() {
        let mut log = EventLog::new();
        
        // Test state operations
        let state = TestState {
            value: "test".to_string(),
        };
        
        StateAccess::store_state(&mut log, &state).await.unwrap();
        
        let retrieved = StateAccess::get_state::<TestState>(&log).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "test");
        
        let deleted = StateAccess::delete_state::<TestState>(&mut log).await.unwrap();
        assert!(deleted.is_some());
        assert_eq!(deleted.unwrap().value, "test");
        
        let retrieved = StateAccess::get_state::<TestState>(&log).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_event_validation() {
        let contract_addr = WasmlAddress::new(vec![1, 2, 3]);

        // Test valid event
        let event = Event::Custom {
            contract_addr: contract_addr.clone(),
            name: "test_event".to_string(),
            data: vec![1, 2, 3],
            height: 1,
            timestamp: 1000,
        };
        let mut log = EventLog::new();
        assert!(log.add_event(event).is_ok());

        // Test name too long
        let long_name = "a".repeat(MAX_EVENT_NAME_LENGTH + 1);
        let event = Event::Custom {
            contract_addr: contract_addr.clone(),
            name: long_name,
            data: vec![1, 2, 3],
            height: 1,
            timestamp: 1000,
        };
        assert!(matches!(log.add_event(event), Err(EventError::NameTooLong(_))));

        // Test data too large
        let large_data = vec![0; MAX_EVENT_DATA_SIZE + 1];
        let event = Event::Custom {
            contract_addr: contract_addr.clone(),
            name: "test_event".to_string(),
            data: large_data,
            height: 1,
            timestamp: 1000,
        };
        assert!(matches!(log.add_event(event), Err(EventError::DataTooLarge(_))));

        // Test state change event
        let event = Event::StateChange {
            key: vec![1; MAX_EVENT_DATA_SIZE / 2],
            value: vec![2; MAX_EVENT_DATA_SIZE / 2 + 1],
        };
        assert!(matches!(log.add_event(event), Err(EventError::DataTooLarge(_))));

        // Test too many events
        let mut log = EventLog::new();
        for _ in 0..MAX_EVENTS_PER_CONTRACT {
            let event = Event::Custom {
                contract_addr: contract_addr.clone(),
                name: "test_event".to_string(),
                data: vec![1, 2, 3],
                height: 1,
                timestamp: 1000,
            };
            assert!(log.add_event(event).is_ok());
        }

        let event = Event::Custom {
            contract_addr: contract_addr.clone(),
            name: "test_event".to_string(),
            data: vec![1, 2, 3],
            height: 1,
            timestamp: 1000,
        };
        assert!(matches!(log.add_event(event), Err(EventError::TooManyEvents(_))));
    }

    #[test]
    fn test_event_log_clear() {
        let mut log = EventLog::new();
        let contract_addr = WasmlAddress::new(vec![1, 2, 3]);

        // Add valid events
        for i in 0..MAX_EVENTS_PER_CONTRACT {
            let event = Event::Custom {
                contract_addr: contract_addr.clone(),
                name: format!("event_{}", i),
                data: vec![i as u8],
                height: 1,
                timestamp: 1000,
            };
            assert!(log.add_event(event).is_ok());
        }

        // Try to add one more event
        let event = Event::Custom {
            contract_addr: contract_addr,
            name: "one_more".to_string(),
            data: vec![0],
            height: 1,
            timestamp: 1000,
        };
        assert!(matches!(log.add_event(event), Err(EventError::TooManyEvents(_))));

        // Test clear
        log.clear();
        assert!(log.events().is_empty());
    }
}
