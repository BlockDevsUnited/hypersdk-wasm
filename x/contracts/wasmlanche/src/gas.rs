#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::string::String;
#[cfg(feature = "std")]
use std::vec::Vec;

use core::num::NonZeroU64;
use crate::error::Error;

// Gas costs for various operations
pub const GAS_BASE_OPERATION: u64 = 1;
pub const GAS_MEMORY_STORE_PER_BYTE: u64 = 1;
pub const GAS_MEMORY_LOAD_PER_BYTE: u64 = 1;
pub const GAS_STATE_STORE_PER_BYTE: u64 = 10;
pub const GAS_STATE_LOAD_PER_BYTE: u64 = 5;
pub const GAS_CONTRACT_CALL_BASE: u64 = 100;
pub const GAS_EVENT_BASE: u64 = 10;
pub const GAS_EVENT_PER_BYTE: u64 = 1;
pub const GAS_CRYPTO_BASE: u64 = 50;
pub const GAS_CRYPTO_PER_BYTE: u64 = 2;

// Limits
pub const MAX_CONTRACT_SIZE: usize = 1024 * 1024; // 1MB
pub const MAX_STATE_KEY_SIZE: usize = 1024; // 1KB
pub const MAX_STATE_VALUE_SIZE: usize = 1024 * 1024; // 1MB
pub const MAX_EVENT_NAME_LENGTH: usize = 64;
pub const MAX_EVENT_DATA_SIZE: usize = 1024 * 1024; // 1MB
pub const MAX_EVENTS_PER_CONTRACT: usize = 100;
pub const MAX_CALL_DEPTH: u32 = 10;
pub const MIN_GAS_LIMIT: u64 = 100_000;
pub const MAX_GAS: u64 = 1_000_000;

#[derive(Debug, Clone)]
pub struct GasCounter {
    remaining: u64,
}

impl Default for GasCounter {
    fn default() -> Self {
        Self {
            remaining: MAX_GAS,
        }
    }
}

impl GasCounter {
    pub fn new(initial_gas: u64) -> Self {
        Self {
            remaining: initial_gas,
        }
    }

    pub fn charge_gas(&mut self, amount: u64) -> Result<(), Error> {
        if amount > self.remaining {
            return Err(Error::Gas("Out of gas".to_string()));
        }
        self.remaining -= amount;
        Ok(())
    }

    pub fn charge_memory(&mut self, bytes: usize) -> Result<(), Error> {
        let gas = (bytes as u64)
            .checked_mul(GAS_MEMORY_STORE_PER_BYTE)
            .ok_or_else(|| Error::TooExpensive("Memory operation too large".into()))?;
        self.charge_gas(gas)
    }
    
    pub fn charge_state_store(&mut self, key_size: usize, value_size: usize) -> Result<(), Error> {
        // Check size limits
        if key_size > MAX_STATE_KEY_SIZE {
            return Err(Error::TooExpensive("State key too large".into()));
        }
        if value_size > MAX_STATE_VALUE_SIZE {
            return Err(Error::TooExpensive("State value too large".into()));
        }
        
        // Calculate gas
        let key_gas = (key_size as u64)
            .checked_mul(GAS_STATE_STORE_PER_BYTE)
            .ok_or_else(|| Error::TooExpensive("State operation too large".into()))?;
            
        let value_gas = (value_size as u64)
            .checked_mul(GAS_STATE_STORE_PER_BYTE)
            .ok_or_else(|| Error::TooExpensive("State operation too large".into()))?;
            
        self.charge_gas(key_gas + value_gas)
    }
    
    pub fn charge_state_load(&mut self, key_size: usize) -> Result<(), Error> {
        if key_size > MAX_STATE_KEY_SIZE {
            return Err(Error::TooExpensive("State key too large".into()));
        }
        
        let gas = (key_size as u64)
            .checked_mul(GAS_STATE_LOAD_PER_BYTE)
            .ok_or_else(|| Error::TooExpensive("State operation too large".into()))?;
            
        self.charge_gas(gas)
    }
    
    pub fn charge_contract_call(&mut self, args_size: usize) -> Result<(), Error> {
        let args_gas = (args_size as u64)
            .checked_mul(GAS_MEMORY_STORE_PER_BYTE)
            .ok_or_else(|| Error::TooExpensive("Contract call args too large".into()))?;
            
        self.charge_gas(GAS_CONTRACT_CALL_BASE + args_gas)
    }
    
    pub fn charge_event(&mut self, name_len: usize, data_size: usize) -> Result<(), Error> {
        if name_len > MAX_EVENT_NAME_LENGTH {
            return Err(Error::TooExpensive("Event name too long".into()));
        }
        if data_size > MAX_EVENT_DATA_SIZE {
            return Err(Error::TooExpensive("Event data too large".into()));
        }
        
        let data_gas = (data_size as u64)
            .checked_mul(GAS_EVENT_PER_BYTE)
            .ok_or_else(|| Error::TooExpensive("Event too large".into()))?;
            
        self.charge_gas(GAS_EVENT_BASE + data_gas)
    }
    
    pub fn charge_crypto(&mut self, input_size: usize) -> Result<(), Error> {
        let data_gas = (input_size as u64)
            .checked_mul(GAS_CRYPTO_PER_BYTE)
            .ok_or_else(|| Error::TooExpensive("Crypto operation too large".into()))?;
            
        self.charge_gas(GAS_CRYPTO_BASE + data_gas)
    }
    
    pub fn gas_remaining(&self) -> u64 {
        self.remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_counter() {
        let mut counter = GasCounter::default();
        assert_eq!(counter.gas_remaining(), MAX_GAS);

        // Test successful gas charge
        counter.charge_gas(100).unwrap();
        assert_eq!(counter.gas_remaining(), MAX_GAS - 100);

        // Test out of gas
        assert!(counter.charge_gas(MAX_GAS).is_err());
    }
}
