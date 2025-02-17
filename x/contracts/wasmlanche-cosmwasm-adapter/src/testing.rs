use std::sync::{Arc, RwLock};
use std::collections::BTreeMap;
use std::ops::Bound;
use cosmwasm_std::{
    Binary, Storage, Api, Querier, QuerierResult, Order,
    Addr, CanonicalAddr, StdError, SystemResult, ContractResult,
    VerificationError, RecoverPubkeyError,
};

#[derive(Clone)]
pub struct ThreadSafeStorage {
    data: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>
}

impl ThreadSafeStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(BTreeMap::new()))
        }
    }
}

impl Default for ThreadSafeStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for ThreadSafeStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.read()
            .unwrap()
            .get(key)
            .cloned()
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.data.write()
            .unwrap()
            .insert(key.to_vec(), value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        self.data.write()
            .unwrap()
            .remove(key);
    }

    fn range<'a>(&'a self, start: Option<&[u8]>, end: Option<&[u8]>, order: Order) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let data = self.data.read().unwrap();
        let start_bound = start.map_or(Bound::Unbounded, |s| Bound::Included(s.to_vec()));
        let end_bound = end.map_or(Bound::Unbounded, |e| Bound::Excluded(e.to_vec()));
        
        // Collect into a Vec to avoid lifetime issues with the RwLockReadGuard
        let items: Vec<_> = data.range((start_bound, end_bound))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let iter = items.into_iter();
        match order {
            Order::Ascending => Box::new(iter),
            Order::Descending => Box::new(iter.rev()),
        }
    }
}

#[derive(Clone)]
pub struct ThreadSafeApi;

impl ThreadSafeApi {
    pub fn new() -> Self {
        Self
    }
}

impl Api for ThreadSafeApi {
    fn debug(&self, message: &str) {
        println!("Debug: {}", message);
    }

    fn addr_validate(&self, human: &str) -> Result<Addr, StdError> {
        Ok(Addr::unchecked(human))
    }

    fn addr_canonicalize(&self, human: &str) -> Result<CanonicalAddr, StdError> {
        Ok(CanonicalAddr::from(Binary::from(human.as_bytes())))
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> Result<Addr, StdError> {
        String::from_utf8(canonical.as_slice().to_vec())
            .map(|s| Addr::unchecked(s))
            .map_err(|_| StdError::generic_err("Invalid canonical address"))
    }

    fn secp256k1_verify(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        Ok(true)
    }

    fn secp256k1_recover_pubkey(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        Ok(vec![])
    }

    fn ed25519_verify(
        &self,
        _message: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        Ok(true)
    }

    fn ed25519_batch_verify(
        &self,
        _messages: &[&[u8]],
        _signatures: &[&[u8]],
        _public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        Ok(true)
    }
}

#[derive(Clone)]
pub struct ThreadSafeQuerier;

impl ThreadSafeQuerier {
    pub fn new() -> Self {
        Self
    }
}

impl Querier for ThreadSafeQuerier {
    fn raw_query(&self, _bin_request: &[u8]) -> QuerierResult {
        SystemResult::Ok(ContractResult::Ok(Binary::from(vec![])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage() {
        let mut storage = ThreadSafeStorage::new();
        
        // Test set and get
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        storage.set(&key, &value);
        
        assert_eq!(storage.get(&key), Some(value.clone()));
        
        // Test remove
        storage.remove(&key);
        assert_eq!(storage.get(&key), None);
        
        // Test range
        let test_data = vec![
            (b"a".to_vec(), b"1".to_vec()),
            (b"b".to_vec(), b"2".to_vec()),
            (b"c".to_vec(), b"3".to_vec()),
        ];
        
        for (k, v) in &test_data {
            storage.set(k, v);
        }
        
        let range_result: Vec<(Vec<u8>, Vec<u8>)> = storage
            .range(Some(b"a"), Some(b"c"), Order::Ascending)
            .collect();
            
        assert_eq!(range_result.len(), 2);
        
        assert_eq!(&range_result[0].0, b"a");
        assert_eq!(&range_result[0].1, b"1");
        assert_eq!(&range_result[1].0, b"b"); 
        assert_eq!(&range_result[1].1, b"2");
    }
}
