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

#[cfg(feature = "std")]
use async_trait::async_trait;

use crate::{
    error::Error,
    state::{StateAccess, StateKey, Error as StateError},
};

// Fixed by adding explicit type parameter for BorshDeserialize
#[derive(Debug, Clone, BorshSerialize)]
#[derive(BorshDeserialize)]
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
    pub fn try_from_slice(slice: &[u8]) -> Result<Self, borsh::maybestd::io::Error>
    {
        borsh::BorshDeserialize::try_from_slice(slice)
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
                    return Err(Error::DataTooLarge(String::from("State change data exceeds maximum size")));
                }
                self.state.insert(key.clone(), value.clone());
            }
            Event::Custom { name, data, .. } => {
                if name.len() > MAX_EVENT_NAME_LENGTH {
                    return Err(Error::NameTooLong(String::from("Event name exceeds maximum length")));
                }
                if data.len() > MAX_EVENT_DATA_SIZE {
                    return Err(Error::DataTooLarge(String::from("Event data exceeds maximum size")));
                }
            }
        }
        
        if self.events.len() >= MAX_EVENTS_PER_CONTRACT {
            return Err(Error::TooManyEvents(String::from("Maximum number of events exceeded")));
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

    pub fn get_state(&self, key: &[u8]) -> Result<Option<Vec<u8>>, EventError> {
        Ok(self.state.get(key).cloned())
    }

    pub fn delete_state(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, EventError> {
        Ok(self.state.remove(key))
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(feature = "std")]
#[async_trait]
impl StateAccess for EventLog {
    async fn store_state<S: BorshSerialize + StateKey + Send + Sync>(&mut self, state: &S) -> Result<(), StateError> {
        let bytes = state.try_to_vec()
            .map_err(|e| StateError::Serialization(e.to_string()))?;
        self.store_state(&S::key(state), &bytes)
            .map_err(|e| StateError::State(e.to_string()))
    }

    async fn get_state<S: BorshDeserialize + StateKey + Default + Send + Sync>(&self) -> Result<Option<S>, StateError> {
        let key = S::key(&S::default());
        let value = self.get_state(&key).map_err(|e| StateError::State(e.to_string()))?;
        match value {
            Some(bytes) => {
                let state = S::try_from_slice(&bytes)
                    .map_err(|e| StateError::Serialization(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    async fn delete_state<S: BorshDeserialize + StateKey + Default + Send + Sync>(&mut self) -> Result<Option<S>, StateError> {
        let key = S::key(&S::default());
        let value = self.get_state(&key).map_err(|e| StateError::State(e.to_string()))?;
        match value {
            Some(bytes) => {
                let state = S::try_from_slice(&bytes)
                    .map_err(|e| StateError::Serialization(e.to_string()))?;
                self.delete_state(&key).map_err(|e| StateError::State(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }
}

#[cfg(not(feature = "std"))]
impl StateAccess for EventLog {
    fn store_state<S: BorshSerialize + StateKey>(&mut self, state: &S) -> Result<(), StateError> {
        let bytes = state.try_to_vec()
            .map_err(|e| StateError::Serialization(e.to_string()))?;
        self.store_state(&S::key(state), &bytes)
            .map_err(|e| StateError::State(e.to_string()))
    }

    fn get_state<S: BorshDeserialize + StateKey + Default>(&self) -> Result<Option<S>, StateError> {
        let key = S::key(&S::default());
        let value = self.get_state(&key).map_err(|e| StateError::State(e.to_string()))?;
        match value {
            Some(bytes) => {
                let state = S::try_from_slice(&bytes)
                    .map_err(|e| StateError::Serialization(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    fn delete_state<S: BorshDeserialize + StateKey + Default>(&mut self) -> Result<Option<S>, StateError> {
        let key = S::key(&S::default());
        let value = self.get_state(&key).map_err(|e| StateError::State(e.to_string()))?;
        match value {
            Some(bytes) => {
                let state = S::try_from_slice(&bytes)
                    .map_err(|e| StateError::Serialization(e.to_string()))?;
                self.delete_state(&key).map_err(|e| StateError::State(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }
}

#[cfg(all(test, feature = "std"))]
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
            "test_state".as_bytes().to_vec()
        }
    }

    #[cfg(feature = "std")]
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

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn test_event_storage() {
        let mut log = EventLog::default();
        
        log.store_state(b"test_key", &b"test_value".to_vec()).unwrap();
        let value = log.get_state(b"test_key").unwrap().unwrap();
        assert_eq!(value, b"test_value".to_vec());
        
        log.delete_state(b"test_key").unwrap();
        let value = log.get_state(b"test_key").unwrap();
        assert!(value.is_none());
    }
}
