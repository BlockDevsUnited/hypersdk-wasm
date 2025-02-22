#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::vec;
#[cfg(not(feature = "std"))]
use alloc::collections::{VecDeque, BTreeMap as HashMap};

#[cfg(feature = "std")]
use std::collections::{VecDeque, HashMap};

use borsh::maybestd::string::ToString as BorshToString;
use borsh::{BorshSerialize, BorshDeserialize};
use crate::error::EventError;
use crate::gas::{MAX_EVENT_NAME_LENGTH, MAX_EVENT_DATA_SIZE, MAX_EVENTS_PER_CONTRACT};
use crate::types::WasmlAddress;

#[cfg(not(target_arch = "wasm32"))]
use async_trait::async_trait;

use crate::{
    error::Error,
    state::{StateAccess, StateKey, Error as StateError},
};

#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize)]
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

impl Event {
    pub fn new(name: &str, data: &str) -> Self {
        Event::Custom {
            contract_addr: WasmlAddress::new([0; 32]),
            name: name.to_string(),
            data: data.as_bytes().to_vec(),
            height: 0,
            timestamp: 0,
        }
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

    pub fn add_event(&mut self, event: Event) -> Result<(), Error> {
        match &event {
            Event::StateChange { key, value } => {
                if key.len() + value.len() > MAX_EVENT_DATA_SIZE {
                    return Err(Error::DataTooLarge("State change data exceeds maximum size"));
                }
                self.state.insert(key.clone(), value.clone());
            }
            Event::Custom { name, data, .. } => {
                if name.len() > MAX_EVENT_NAME_LENGTH {
                    return Err(Error::NameTooLong("Event name exceeds maximum length"));
                }
                if data.len() > MAX_EVENT_DATA_SIZE {
                    return Err(Error::DataTooLarge("Event data exceeds maximum size"));
                }
            }
        }
        
        if self.events.len() >= MAX_EVENTS_PER_CONTRACT {
            return Err(Error::TooManyEvents("Maximum number of events exceeded"));
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
    async fn store_state<S: BorshSerialize + StateKey + Send + Sync>(&mut self, state: &S) -> Result<(), StateError> {
        let bytes = state.try_to_vec()
            .map_err(|e| StateError::Serialization(e.to_string()))?;
        self.store_state(&S::key(state), &bytes)
            .map_err(|e| StateError::State(e.to_string()))
    }

    async fn get_state<S: BorshDeserialize + StateKey + Send + Sync>(&self) -> Result<Option<S>, StateError> {
        match self.get_state(&S::key(&S::default())) {
            Some(bytes) => {
                S::try_from_slice(bytes)
                    .map(Some)
                    .map_err(|e| StateError::Serialization(e.to_string()))
            }
            None => Ok(None),
        }
    }

    async fn delete_state<S: BorshDeserialize + StateKey + Send + Sync>(&mut self) -> Result<Option<S>, StateError> {
        match self.delete_state(&S::key(&S::default())) {
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

    #[derive(BorshSerialize, BorshDeserialize, Default)]
    struct TestState {
        value: u64,
    }

    impl BorshToString for TestState {
        fn to_string(&self) -> String {
            format!("TestState({})", self.value)
        }
    }

    impl StateKey for TestState {
        fn key(&self) -> Vec<u8> {
            b"test_state".to_vec()
        }
    }

    #[tokio::test]
    async fn test_event_log() {
        let mut log = EventLog::default();
        let contract_addr = WasmlAddress::new([1; 32]);

        let state = TestState { value: 42 };
        StateAccess::store_state(&mut log, &state).await.unwrap();

        let state = StateAccess::get_state::<TestState>(&log).await.unwrap();
        assert_eq!(state.unwrap().value, 42);

        StateAccess::delete_state::<TestState>(&mut log).await.unwrap();

        let state = StateAccess::get_state::<TestState>(&log).await.unwrap();
        assert!(state.is_none());

        let event = Event::StateChange {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        };

        log.add_event(event).unwrap();
        assert_eq!(log.events().len(), 1);
    }

    #[tokio::test]
    async fn test_event_log_with_contract() {
        let mut log = EventLog::default();
        let contract_addr = WasmlAddress::new([1; 32]);

        let event = Event::StateChange {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        };

        log.add_event(event).unwrap();
        assert_eq!(log.events().len(), 1);
    }

    #[test]
    fn test_event_validation() {
        let contract_addr = WasmlAddress::new([1; 32]);

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
        assert!(matches!(log.add_event(event), Err(Error::NameTooLong(_))));

        // Test data too large
        let large_data = vec![0; MAX_EVENT_DATA_SIZE + 1];
        let event = Event::Custom {
            contract_addr: contract_addr.clone(),
            name: "test_event".to_string(),
            data: large_data,
            height: 1,
            timestamp: 1000,
        };
        assert!(matches!(log.add_event(event), Err(Error::DataTooLarge(_))));

        // Test state change event
        let event = Event::StateChange {
            key: vec![1; MAX_EVENT_DATA_SIZE / 2],
            value: vec![2; MAX_EVENT_DATA_SIZE / 2 + 1],
        };
        assert!(matches!(log.add_event(event), Err(Error::DataTooLarge(_))));

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
        assert!(matches!(log.add_event(event), Err(Error::TooManyEvents(_))));
    }

    #[test]
    fn test_event_log_clear() {
        let mut log = EventLog::new();
        let contract_addr = WasmlAddress::new([1; 32]);

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
        assert!(matches!(log.add_event(event), Err(Error::TooManyEvents(_))));

        // Test clear
        log.clear();
        assert!(log.events().is_empty());
    }
}
