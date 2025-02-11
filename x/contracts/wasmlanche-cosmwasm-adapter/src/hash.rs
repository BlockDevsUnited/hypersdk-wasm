use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use borsh::BorshSerialize;

/// Tracks state changes and calculates state hash
pub struct StateHasher {
    changes: BTreeMap<String, Option<Vec<u8>>>,
}

impl StateHasher {
    pub fn new() -> Self {
        Self {
            changes: BTreeMap::new(),
        }
    }

    /// Record a state change
    pub fn record_change(&mut self, key: String, value: Option<Vec<u8>>) {
        self.changes.insert(key, value);
    }

    /// Calculate state hash from all recorded changes
    pub fn calculate_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        
        // Sort changes by key for deterministic hashing
        for (key, value) in &self.changes {
            // Hash key
            hasher.update(key.as_bytes());
            
            // Hash value or deletion marker
            match value {
                Some(v) => {
                    hasher.update(&[1u8]); // Exists marker
                    hasher.update(&v);
                }
                None => {
                    hasher.update(&[0u8]); // Deleted marker
                }
            }
        }
        
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_hasher() {
        let mut hasher = StateHasher::new();
        
        // Test adding values
        hasher.record_change("key1".to_string(), Some(vec![1, 2, 3]));
        hasher.record_change("key2".to_string(), Some(vec![4, 5, 6]));
        
        let hash1 = hasher.calculate_hash();
        
        // Test deterministic ordering
        let mut hasher2 = StateHasher::new();
        hasher2.record_change("key2".to_string(), Some(vec![4, 5, 6]));
        hasher2.record_change("key1".to_string(), Some(vec![1, 2, 3]));
        
        let hash2 = hasher2.calculate_hash();
        
        assert_eq!(hash1, hash2, "Hashes should be equal regardless of insertion order");
        
        // Test deletions
        hasher.record_change("key1".to_string(), None);
        let hash3 = hasher.calculate_hash();
        assert_ne!(hash1, hash3, "Hash should change after deletion");
    }
}
