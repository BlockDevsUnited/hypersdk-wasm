use std::sync::{Arc, RwLock};
use cosmwasm_std::{Storage, Order, Api, Querier, Binary, ContractResult, SystemResult, VerificationError, RecoverPubkeyError, Addr};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

const CONTRACT_STATE_PREFIX: &[u8] = b"contract_state/";
const CONTRACT_CODE_PREFIX: &[u8] = b"contract_code/";
const CONTRACT_ADMIN_PREFIX: &[u8] = b"contract_admin/";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractInfo {
    pub code_id: u64,
    pub creator: Addr,
    pub admin: Option<Addr>,
    pub label: String,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CodeInfo {
    pub creator: Addr,
    pub checksum: [u8; 32],
    pub created_at: u64,
}

pub struct ContractState<'a> {
    storage: &'a mut dyn Storage,
    contract_addr: Addr,
    pub(crate) info: ContractInfo,
}

impl<'a> ContractState<'a> {
    pub fn new(
        storage: &'a mut dyn Storage,
        contract_addr: Addr,
        code_id: u64,
        creator: Addr,
        admin: Option<Addr>,
        label: String,
    ) -> Self {
        let info = ContractInfo {
            code_id,
            creator,
            admin,
            label,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        Self {
            storage,
            contract_addr,
            info,
        }
    }

    pub fn load(storage: &'a mut dyn Storage, contract_addr: Addr) -> Option<Self> {
        let key = [CONTRACT_STATE_PREFIX, contract_addr.as_bytes()].concat();
        let info: ContractInfo = storage.get(&key)
            .and_then(|data| serde_json::from_slice(&data).ok())?;
        
        Some(Self {
            storage,
            contract_addr,
            info,
        })
    }

    pub fn save(&mut self) -> Result<(), cosmwasm_std::StdError> {
        let key = [CONTRACT_STATE_PREFIX, self.contract_addr.as_bytes()].concat();
        let data = serde_json::to_vec(&self.info)
            .map_err(|e| cosmwasm_std::StdError::serialize_err("ContractInfo", e))?;
        self.storage.set(&key, &data);
        Ok(())
    }

    pub fn get_storage(&mut self) -> StorageAdapter {
        StorageAdapter::new(
            self.storage,
            [CONTRACT_STATE_PREFIX, self.contract_addr.as_bytes()].concat(),
        )
    }

    pub fn update_admin(&mut self, new_admin: Option<Addr>) -> Result<(), cosmwasm_std::StdError> {
        self.info.admin = new_admin;
        self.save()
    }

    pub fn get_code_info(&self) -> Option<CodeInfo> {
        let key = [CONTRACT_CODE_PREFIX, &self.info.code_id.to_be_bytes()].concat();
        self.storage.get(&key)
            .and_then(|data| serde_json::from_slice(&data).ok())
    }

    pub fn get_info(&self) -> &ContractInfo {
        &self.info
    }
}

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
            .map_err(|e| cosmwasm_std::StdError::serialize_err(key, e))?;

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
                .map_err(|e| cosmwasm_std::StdError::serialize_err("ContractState", e))
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn test_contract_state() {
        let mut storage = MockStorage::default();
        let contract_addr = Addr::unchecked("contract1");
        let creator = Addr::unchecked("creator");
        let admin = Some(Addr::unchecked("admin"));
        let label = "My Contract".to_string();

        // Create new contract state
        let mut contract = ContractState::new(
            &mut storage,
            contract_addr.clone(),
            1u64,
            creator.clone(),
            admin.clone(),
            label.clone(),
        );
        contract.save().unwrap();

        // Load contract state
        let loaded = ContractState::load(&mut storage, contract_addr.clone()).unwrap();
        assert_eq!(loaded.get_info().code_id, 1u64);
        assert_eq!(loaded.get_info().creator, creator);
        assert_eq!(loaded.get_info().admin, admin);
        assert_eq!(loaded.get_info().label, label);

        // Update admin
        let mut contract = ContractState::load(&mut storage, contract_addr.clone()).unwrap();
        let new_admin = Some(Addr::unchecked("new_admin"));
        contract.update_admin(new_admin.clone()).unwrap();

        // Verify admin update
        let loaded = ContractState::load(&mut storage, contract_addr).unwrap();
        assert_eq!(loaded.get_info().admin, new_admin);
    }

    #[test]
    fn test_contract_storage() {
        let mut storage = MockStorage::default();
        let contract_addr = Addr::unchecked("contract1");
        let creator = Addr::unchecked("creator");
        
        // Create contract state
        let mut contract = ContractState::new(
            &mut storage,
            contract_addr.clone(),
            1u64,
            creator,
            None,
            "test".to_string(),
        );
        contract.save().unwrap();

        // Get contract storage
        let mut contract_storage = contract.get_storage();

        // Test state operations
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestState {
            value: String,
        }

        let test_state = TestState {
            value: "test value".to_string(),
        };

        // Set state
        contract_storage.set_state("test_key", &test_state).unwrap();

        // Get state
        let loaded: TestState = contract_storage.get_state("test_key").unwrap();
        assert_eq!(loaded, test_state);

        // Delete state
        contract_storage.delete_state("test_key");
        assert!(contract_storage.get_state::<TestState>("test_key").is_none());
    }
}
