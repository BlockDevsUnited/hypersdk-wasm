use std::sync::{Arc, RwLock};
use cosmwasm_std::{Api, Storage, Querier, VerificationError, RecoverPubkeyError};

use crate::error::HostError;

pub type HostResult<T> = Result<T, HostError>;

const GAS_COST_READ_MEMORY: u64 = 10;
const GAS_COST_WRITE_MEMORY: u64 = 20;
const GAS_COST_ALLOCATE: u64 = 30;
const GAS_COST_DEALLOCATE: u64 = 10;

pub struct HostEnv<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    storage: Arc<RwLock<S>>,
    api: Arc<A>,
    querier: Arc<Q>,
    memory: Vec<u8>,
    gas_limit: u64,
    gas_counter: Arc<RwLock<u64>>,
}

impl<S, A, Q> HostEnv<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    pub fn new(storage: S, api: A, querier: Q, gas_limit: u64) -> Self {
        Self {
            storage: Arc::new(RwLock::new(storage)),
            api: Arc::new(api),
            querier: Arc::new(querier),
            memory: Vec::new(),
            gas_limit,
            gas_counter: Arc::new(RwLock::new(0)),
        }
    }

    pub fn charge_gas(&self, amount: u64) -> HostResult<()> {
        let mut counter = self.gas_counter.write().map_err(|_| {
            HostError::GasLimit("Failed to acquire gas counter lock".to_string())
        })?;

        let new_counter = counter.checked_add(amount)
            .ok_or_else(|| HostError::GasLimit("Gas counter overflow".to_string()))?;

        if new_counter > self.gas_limit {
            return Err(HostError::GasLimit("Gas limit exceeded".to_string()));
        }

        *counter = new_counter;
        Ok(())
    }

    pub fn get_gas_used(&self) -> HostResult<u64> {
        self.gas_counter.read()
            .map_err(|_| HostError::GasLimit("Failed to read gas counter".to_string()))
            .map(|counter| *counter)
    }

    pub fn allocate(&mut self, size: usize) -> HostResult<usize> {
        self.charge_gas(GAS_COST_ALLOCATE)?;
        
        let current_len = self.memory.len();
        self.memory.resize(current_len + size, 0);
        Ok(current_len)
    }

    pub fn deallocate(&mut self, ptr: usize) -> HostResult<()> {
        self.charge_gas(GAS_COST_DEALLOCATE)?;
        
        if ptr >= self.memory.len() {
            return Err(HostError::MemoryAccess(format!("Invalid pointer: {}", ptr)));
        }
        Ok(())
    }

    pub fn read_memory(&self, offset: usize, len: usize) -> HostResult<&[u8]> {
        self.charge_gas(GAS_COST_READ_MEMORY)?;
        
        let end = offset.checked_add(len)
            .ok_or_else(|| HostError::MemoryAccess("Memory access overflow".to_string()))?;

        if end > self.memory.len() {
            return Err(HostError::MemoryAccess("Memory access out of bounds".to_string()));
        }

        Ok(&self.memory[offset..end])
    }

    pub fn write_memory(&mut self, offset: usize, data: &[u8]) -> HostResult<()> {
        self.charge_gas(GAS_COST_WRITE_MEMORY)?;
        
        let end = offset.checked_add(data.len())
            .ok_or_else(|| HostError::MemoryAccess("Memory access overflow".to_string()))?;

        if end > self.memory.len() {
            return Err(HostError::MemoryAccess("Memory access out of bounds".to_string()));
        }

        self.memory[offset..end].copy_from_slice(data);
        Ok(())
    }

    pub fn ptr_to_usize(&self, ptr: u32) -> HostResult<usize> {
        usize::try_from(ptr).map_err(|_| {
            HostError::MemoryAccess(format!("Invalid pointer conversion: {}", ptr))
        })
    }

    pub fn ptr_to_slice(&self, ptr: usize) -> HostResult<&[u8]> {
        if ptr >= self.memory.len() {
            return Err(HostError::MemoryAccess(format!("Invalid pointer: {}", ptr)));
        }
        Ok(&self.memory[ptr..])
    }

    pub fn ptr_to_slice_mut(&mut self, ptr: usize) -> HostResult<&mut [u8]> {
        if ptr >= self.memory.len() {
            return Err(HostError::MemoryAccess(format!("Invalid pointer: {}", ptr)));
        }
        Ok(&mut self.memory[ptr..])
    }

    pub fn storage(&self) -> HostResult<std::sync::RwLockReadGuard<'_, S>> {
        self.storage.read().map_err(|_| {
            HostError::Storage("Failed to acquire storage lock".to_string())
        })
    }

    pub fn storage_mut(&self) -> HostResult<std::sync::RwLockWriteGuard<'_, S>> {
        self.storage.write().map_err(|_| {
            HostError::Storage("Failed to acquire storage lock".to_string())
        })
    }

    pub fn api(&self) -> Arc<A> {
        Arc::clone(&self.api)
    }

    pub fn querier(&self) -> Arc<Q> {
        Arc::clone(&self.querier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{VerificationError, RecoverPubkeyError};
    use std::collections::HashMap;

    #[derive(Clone, Default)]
    pub struct CloneableStorage {
        data: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>
    }

    impl Storage for CloneableStorage {
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
            order: cosmwasm_std::Order,
        ) -> Box<dyn Iterator<Item = cosmwasm_std::Record> + 'a> {
            let data = self.data.read().unwrap();
            let iter = data.iter()
                .filter(move |(k, _)| {
                    let valid_start = start.map_or(true, |s| k.as_slice() >= s);
                    let valid_end = end.map_or(true, |e| k.as_slice() < e);
                    valid_start && valid_end
                })
                .map(|(k, v)| (k.clone(), v.clone()));

            match order {
                cosmwasm_std::Order::Ascending => Box::new(iter.collect::<Vec<_>>().into_iter()),
                cosmwasm_std::Order::Descending => Box::new(iter.collect::<Vec<_>>().into_iter().rev()),
            }
        }
    }

    #[derive(Clone, Default)]
    struct CloneableApi;

    impl Api for CloneableApi {
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
    struct CloneableQuerier;

    impl Querier for CloneableQuerier {
        fn raw_query(&self, _bin_request: &[u8]) -> cosmwasm_std::QuerierResult {
            cosmwasm_std::SystemResult::Ok(cosmwasm_std::ContractResult::Ok(cosmwasm_std::Binary::default()))
        }
    }

    #[test]
    fn test_gas_charging() {
        let env = HostEnv::new(
            CloneableStorage::default(),
            CloneableApi::default(),
            CloneableQuerier::default(),
            1000,
        );
        
        // Test successful gas charging
        assert!(env.charge_gas(500).is_ok());
        assert!(env.charge_gas(400).is_ok());
        
        // Test gas limit exceeded
        assert!(env.charge_gas(200).is_err());
        
        // Verify gas usage
        assert_eq!(env.get_gas_used().unwrap(), 900);
    }

    #[test]
    fn test_memory_operations() {
        let mut env = HostEnv::new(
            CloneableStorage::default(),
            CloneableApi::default(),
            CloneableQuerier::default(),
            1000,
        );
        
        // Test allocation
        let offset = env.allocate(10).unwrap();
        
        // Write to memory
        let data = vec![1, 2, 3, 4, 5];
        env.write_memory(offset, &data).unwrap();
        
        // Read from memory
        let read_data = env.read_memory(offset, data.len()).unwrap();
        assert_eq!(read_data, &data);
        
        // Test deallocation
        assert!(env.deallocate(offset).is_ok());
        
        // Verify gas usage includes memory operations
        let gas_used = env.get_gas_used().unwrap();
        assert!(gas_used > 0);
    }

    #[test]
    fn test_storage_access() {
        let env = HostEnv::new(
            CloneableStorage::default(),
            CloneableApi::default(),
            CloneableQuerier::default(),
            1000,
        );
        
        // Test storage read access
        let storage = env.storage().unwrap();
        assert!(storage.get(b"test").is_none());
        drop(storage);
        
        // Test storage write access
        let mut storage = env.storage_mut().unwrap();
        storage.set(b"test", b"value");
        drop(storage);
        
        // Verify written value
        let storage = env.storage().unwrap();
        assert_eq!(storage.get(b"test").unwrap(), b"value");
    }
}
