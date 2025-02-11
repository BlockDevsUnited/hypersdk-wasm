use std::sync::{Arc, RwLock};
use cosmwasm_std::{Storage, Order, Api, Querier, Binary, ContractResult, SystemResult, VerificationError, RecoverPubkeyError};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

pub struct StorageAdapter<'a> {
    storage: &'a mut dyn Storage,
    prefix: Vec<u8>,
    state_hasher: Arc<RwLock<Sha256>>,
}

impl<'a> StorageAdapter<'a> {
    pub fn new(storage: &'a mut dyn Storage, prefix: Vec<u8>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&prefix);
        Self {
            storage,
            prefix,
            state_hasher: Arc::new(RwLock::new(hasher)),
        }
    }

    fn get_prefixed_key(&self, key: &str) -> Vec<u8> {
        let mut prefixed_key = self.prefix.clone();
        prefixed_key.extend_from_slice(key.as_bytes());
        prefixed_key
    }

    pub fn set_state<T: Serialize>(&mut self, key: &str, value: &T) -> Result<(), cosmwasm_std::StdError> {
        // Serialize value
        let serialized = serde_json::to_vec(value)
            .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))?;

        // Get prefixed key
        let prefixed_key = self.get_prefixed_key(key);

        // Update state hash
        let mut hasher = self.state_hasher.write().unwrap();
        hasher.update(&prefixed_key);
        hasher.update(&serialized);

        // Store value
        self.storage.set(&prefixed_key, &serialized);
        Ok(())
    }

    pub fn get_state<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let prefixed_key = self.get_prefixed_key(key);
        self.storage.get(&prefixed_key).and_then(|data| {
            serde_json::from_slice(&data)
                .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))
                .ok()
        })
    }

    pub fn delete_state(&mut self, key: &str) {
        let prefixed_key = self.get_prefixed_key(key);
        self.storage.remove(&prefixed_key);

        // Update state hash
        let mut hasher = self.state_hasher.write().unwrap();
        hasher.update(&prefixed_key);
    }

    pub fn calculate_state_hash(&self) -> [u8; 32] {
        let hasher = self.state_hasher.read().unwrap();
        let result = hasher.clone().finalize();
        result.into()
    }
}

impl Storage for StorageAdapter<'_> {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.storage.get(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.storage.set(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.storage.remove(key)
    }

    fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        self.storage.range(start, end, order)
    }
}

#[derive(Clone, Default)]
pub struct MockStorage {
    data: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl Storage for MockStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.read().unwrap().get(&key.to_vec()).cloned()
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.data.write().unwrap().insert(key.to_vec(), value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        self.data.write().unwrap().remove(&key.to_vec());
    }

    fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let data = self.data.read().unwrap();
        let iter = data.iter()
            .filter(move |(k, _)| {
                let valid_start = start.map_or(true, |s| k.as_slice() >= s);
                let valid_end = end.map_or(true, |e| k.as_slice() < e);
                valid_start && valid_end
            })
            .map(|(k, v)| (k.clone(), v.clone()));

        match order {
            Order::Ascending => Box::new(iter.collect::<Vec<_>>().into_iter()),
            Order::Descending => Box::new(iter.collect::<Vec<_>>().into_iter().rev()),
        }
    }
}

#[derive(Clone)]
pub struct ThreadSafeStorage(Arc<RwLock<MockStorage>>);

impl Default for ThreadSafeStorage {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(MockStorage::default())))
    }
}

impl Storage for ThreadSafeStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.read().unwrap().get(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.0.write().unwrap().set(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.0.write().unwrap().remove(key)
    }

    fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let data = self.0.read().unwrap();
        let items: Vec<_> = data.range(start, end, order)
            .map(|(k, v)| (k.to_vec(), v.to_vec()))
            .collect();
        Box::new(items.into_iter())
    }
}

#[derive(Clone)]
pub struct ThreadSafeQuerier(Arc<MockQuerier>);

impl Default for ThreadSafeQuerier {
    fn default() -> Self {
        Self(Arc::new(MockQuerier::default()))
    }
}

impl Querier for ThreadSafeQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> cosmwasm_std::QuerierResult {
        self.0.raw_query(bin_request)
    }
}

#[derive(Clone, Default)]
pub struct MockApi;

impl Api for MockApi {
    fn addr_validate(&self, human: &str) -> cosmwasm_std::StdResult<cosmwasm_std::Addr> {
        Ok(cosmwasm_std::Addr::unchecked(human))
    }

    fn addr_canonicalize(&self, human: &str) -> cosmwasm_std::StdResult<cosmwasm_std::CanonicalAddr> {
        Ok(cosmwasm_std::CanonicalAddr::from(human.as_bytes()))
    }

    fn addr_humanize(&self, canonical: &cosmwasm_std::CanonicalAddr) -> cosmwasm_std::StdResult<cosmwasm_std::Addr> {
        Ok(cosmwasm_std::Addr::unchecked(String::from_utf8_lossy(canonical.as_slice())))
    }

    fn secp256k1_verify(&self, _message_hash: &[u8], _signature: &[u8], _public_key: &[u8]) -> Result<bool, VerificationError> {
        Ok(true)
    }

    fn secp256k1_recover_pubkey(&self, _message_hash: &[u8], _signature: &[u8], _recovery_param: u8) -> Result<Vec<u8>, RecoverPubkeyError> {
        Ok(vec![])
    }

    fn ed25519_verify(&self, _message: &[u8], _signature: &[u8], _public_key: &[u8]) -> Result<bool, VerificationError> {
        Ok(true)
    }

    fn ed25519_batch_verify(&self, _messages: &[&[u8]], _signatures: &[&[u8]], _public_keys: &[&[u8]]) -> Result<bool, VerificationError> {
        Ok(true)
    }

    fn debug(&self, _message: &str) {
        // No-op for tests
    }
}

#[derive(Clone, Default)]
pub struct MockQuerier;

impl Querier for MockQuerier {
    fn raw_query(&self, _bin_request: &[u8]) -> cosmwasm_std::QuerierResult {
        SystemResult::Ok(ContractResult::Ok(Binary::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_operations() {
        let storage = Arc::new(RwLock::new(MockStorage::default()));
        
        // Test write
        {
            let mut guard = storage.write().unwrap();
            let mut adapter = StorageAdapter::new(&mut *guard, b"test".to_vec());
            adapter.set_state("counter", &42i32).unwrap();
        }

        // Test read
        {
            let mut guard = storage.write().unwrap();
            let adapter = StorageAdapter::new(&mut *guard, b"test".to_vec());
            let value: i32 = adapter.get_state("counter").unwrap();
            assert_eq!(value, 42);
        }

        // Test delete
        {
            let mut guard = storage.write().unwrap();
            let mut adapter = StorageAdapter::new(&mut *guard, b"test".to_vec());
            adapter.delete_state("counter");
        }

        // Verify deletion
        {
            let mut guard = storage.write().unwrap();
            let adapter = StorageAdapter::new(&mut *guard, b"test".to_vec());
            assert!(adapter.get_state::<i32>("counter").is_none());
        }
    }

    #[test]
    fn test_prefix_isolation() {
        let storage = Arc::new(RwLock::new(MockStorage::default()));
        
        // Write with first adapter
        {
            let mut guard = storage.write().unwrap();
            let mut adapter = StorageAdapter::new(&mut *guard, b"test1".to_vec());
            adapter.set_state("key", &1i32).unwrap();
        }
        
        // Write with second adapter
        {
            let mut guard = storage.write().unwrap();
            let mut adapter = StorageAdapter::new(&mut *guard, b"test2".to_vec());
            adapter.set_state("key", &2i32).unwrap();
        }

        // Read and verify
        {
            let mut guard = storage.write().unwrap();
            let adapter1 = StorageAdapter::new(&mut *guard, b"test1".to_vec());
            let value1: i32 = adapter1.get_state("key").unwrap();
            assert_eq!(value1, 1);
        }
        {
            let mut guard = storage.write().unwrap();
            let adapter2 = StorageAdapter::new(&mut *guard, b"test2".to_vec());
            let value2: i32 = adapter2.get_state("key").unwrap();
            assert_eq!(value2, 2);
        }
    }

    #[test]
    fn test_range() {
        let storage = Arc::new(RwLock::new(MockStorage::default()));
        
        // Insert test data
        {
            let mut guard = storage.write().unwrap();
            let mut adapter = StorageAdapter::new(&mut *guard, b"test".to_vec());
            for i in 0..5 {
                let key = format!("key{}", i);
                adapter.set_state(&key, &i).unwrap();
            }
        }

        // Test range query
        let items = {
            let mut guard = storage.write().unwrap();
            let adapter = StorageAdapter::new(&mut *guard, b"test".to_vec());
            let mut items: Vec<_> = adapter.storage.range(None, None, Order::Ascending)
                .map(|(k, v)| {
                    let key = String::from_utf8(k[4..].to_vec()).unwrap(); // Skip prefix "test"
                    let value: i32 = serde_json::from_slice(&v).unwrap();
                    (key, value)
                })
                .collect();
            items.sort_by(|a, b| a.0.cmp(&b.0));
            items
        };

        assert_eq!(items.len(), 5);
        for (i, (key, value)) in items.iter().enumerate() {
            assert_eq!(key, &format!("key{}", i));
            assert_eq!(*value, i as i32);
        }
    }
}
