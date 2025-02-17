use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use cosmwasm_std::Storage;
use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Code not found for id: {0}")]
    CodeNotFound(u64),
    
    #[error("Code already exists with id: {0}")]
    CodeExists(u64),
    
    #[error("Invalid code: {0}")]
    InvalidCode(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCode {
    pub id: u64,
    pub code: Vec<u8>,
    pub checksum: [u8; 32],
}

#[derive(Debug)]
pub struct CodeStorage {
    codes: Arc<RwLock<HashMap<u64, ContractCode>>>,
    next_id: Arc<RwLock<u64>>,
}

impl CodeStorage {
    pub fn new() -> Self {
        Self {
            codes: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    pub fn store_code(&self, code: Vec<u8>) -> Result<u64, StorageError> {
        // Calculate checksum
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&code);
        let checksum = hasher.finalize().into();

        // Get next ID
        let id = {
            let mut id_guard = self.next_id.write().map_err(|_| 
                StorageError::StorageError("Failed to acquire write lock for ID".to_string()))?;
            let id = *id_guard;
            *id_guard += 1;
            id
        };

        // Store code
        let contract_code = ContractCode {
            id,
            code,
            checksum,
        };

        let mut codes = self.codes.write().map_err(|_| 
            StorageError::StorageError("Failed to acquire write lock for codes".to_string()))?;
        
        codes.insert(id, contract_code);
        
        Ok(id)
    }

    pub fn get_code(&self, id: u64) -> Result<Vec<u8>, StorageError> {
        let codes = self.codes.read().map_err(|_| 
            StorageError::StorageError("Failed to acquire read lock".to_string()))?;
        
        codes.get(&id)
            .map(|code| code.code.clone())
            .ok_or(StorageError::CodeNotFound(id))
    }

    pub fn remove_code(&self, id: u64) -> Result<(), StorageError> {
        let mut codes = self.codes.write().map_err(|_| 
            StorageError::StorageError("Failed to acquire write lock".to_string()))?;
        
        codes.remove(&id)
            .map(|_| ())
            .ok_or(StorageError::CodeNotFound(id))
    }

    pub fn verify_code(&self, id: u64, checksum: &[u8; 32]) -> Result<bool, StorageError> {
        let codes = self.codes.read().map_err(|_| 
            StorageError::StorageError("Failed to acquire read lock".to_string()))?;
        
        codes.get(&id)
            .map(|code| code.checksum == *checksum)
            .ok_or(StorageError::CodeNotFound(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_storage() {
        let storage = CodeStorage::new();
        
        // Store code
        let code = vec![1, 2, 3, 4];
        let id = storage.store_code(code.clone()).unwrap();
        
        // Get code
        let retrieved = storage.get_code(id).unwrap();
        assert_eq!(retrieved, code);
        
        // Verify code
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&code);
        let checksum: [u8; 32] = hasher.finalize().into();
        
        assert!(storage.verify_code(id, &checksum).unwrap());
        
        // Remove code
        storage.remove_code(id).unwrap();
        assert!(storage.get_code(id).is_err());
    }
}
