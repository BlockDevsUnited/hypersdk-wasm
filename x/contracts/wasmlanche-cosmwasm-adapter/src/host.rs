use std::cell::RefCell;
use wasmtime::{Memory, Store};
use cosmwasm_std::{Storage, Api, Querier, CanonicalAddr};
use crate::error::ExecutorError;

pub struct HostEnv<S, A, Q>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    pub(crate) storage: S,
    pub(crate) api: A,
    pub(crate) querier: Q,
    pub(crate) memory: Option<Memory>,
    gas_used: RefCell<u64>,
    gas_limit: u64,
    pub(crate) next_ptr: RefCell<u32>,
}

impl<S, A, Q> HostEnv<S, A, Q>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    pub fn new(storage: S, api: A, querier: Q, gas_limit: u64) -> Self {
        Self {
            storage,
            api,
            querier,
            memory: None,
            gas_used: RefCell::new(0),
            gas_limit,
            next_ptr: RefCell::new(65536), // Start at 64KB to avoid conflicts with other regions
        }
    }

    pub fn set_memory(&mut self, memory: Memory) {
        self.memory = Some(memory);
    }

    pub fn set_gas_limit(&mut self, gas_limit: u64) {
        self.gas_limit = gas_limit;
    }

    pub fn charge_gas(&self, amount: u64) -> Result<(), ExecutorError> {
        let mut gas_used = self.gas_used.borrow_mut();
        *gas_used += amount;
        if *gas_used > self.gas_limit {
            return Err(ExecutorError::GasLimitExceeded);
        }
        Ok(())
    }

    pub fn allocate(&mut self, size: u32) -> anyhow::Result<u32> {
        let mut next_ptr = self.next_ptr.borrow_mut();
        let ptr = *next_ptr;
        
        // Add 4 bytes for length prefix, then the actual data size
        let total_size = size.checked_add(4)
            .ok_or_else(|| anyhow::anyhow!("Memory size overflow"))?;
        
        let new_ptr = next_ptr.checked_add(total_size)
            .ok_or_else(|| anyhow::anyhow!("Memory size overflow"))?;
            
        *next_ptr = new_ptr;
        Ok(ptr + 4)
    }

    pub fn deallocate(&mut self, _ptr: u32) -> anyhow::Result<()> {
        // For now, we don't actually deallocate memory
        Ok(())
    }

    pub fn addr_validate(&self, addr: &str) -> Result<(), ExecutorError> {
        self.charge_gas(100)?;
        self.api.addr_validate(addr)
            .map_err(|e| ExecutorError::ApiError(e.to_string()))?;
        Ok(())
    }

    pub fn addr_canonicalize(&self, human: &str) -> Result<CanonicalAddr, ExecutorError> {
        self.charge_gas(100)?;
        self.api.addr_canonicalize(human)
            .map_err(|e| ExecutorError::ApiError(e.to_string()))
    }

    pub fn addr_humanize(&self, canonical: &[u8]) -> Result<String, ExecutorError> {
        self.charge_gas(100)?;
        self.api.addr_humanize(&CanonicalAddr::from(canonical))
            .map_err(|e| ExecutorError::ApiError(e.to_string()))
            .map(|addr| addr.to_string())
    }
}

pub fn write_memory<S, A, Q>(
    store: &mut Store<HostEnv<S, A, Q>>,
    data: &[u8],
) -> Result<(usize, usize), ExecutorError>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    let memory = {
        store.data().memory.as_ref()
            .ok_or_else(|| ExecutorError::MemoryAccessError("No memory available".to_string()))?
            .clone()
    };

    let total_size = data.len() + 4;
    let ptr = store.data().next_ptr.borrow().clone() as usize;
    
    // Calculate required pages
    let required_pages = ((total_size + ptr + 65535) / 65536) as u64;
    
    // Get current pages and grow if needed
    let pages = memory.size(&mut *store);
    if required_pages > pages {
        memory.grow(&mut *store, required_pages - pages)
            .map_err(|e| ExecutorError::MemoryAccessError(format!("Failed to grow memory: {}", e)))?;
    }

    // Write length prefix
    let len_bytes = (data.len() as u32).to_be_bytes();
    memory.write(&mut *store, ptr, &len_bytes)
        .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;

    // Write data
    memory.write(&mut *store, ptr + 4, data)
        .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;

    // Update next_ptr
    *store.data().next_ptr.borrow_mut() = (ptr + total_size) as u32;

    Ok((ptr, data.len()))
}

pub fn read_memory<S, A, Q>(
    store: &mut Store<HostEnv<S, A, Q>>,
    ptr: usize,
    max_length: usize,
) -> Result<Vec<u8>, ExecutorError>
where
    S: Storage,
    A: Api,
    Q: Querier,
{
    let memory = {
        store.data().memory.as_ref()
            .ok_or_else(|| ExecutorError::MemoryAccessError("No memory available".to_string()))?
            .clone()
    };

    // Read length prefix (32-bit big-endian)
    let mut len_bytes = [0u8; 4];
    memory.read(&mut *store, ptr, &mut len_bytes)
        .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;
    let actual_len = u32::from_be_bytes(len_bytes) as usize;

    // Ensure length is within bounds
    if actual_len > max_length {
        return Err(ExecutorError::MemoryAccessError(format!("Length {} exceeds maximum {}", actual_len, max_length)));
    }

    // Read the actual data
    let mut data = vec![0u8; actual_len];
    memory.read(&mut *store, ptr + 4, &mut data)
        .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Engine, MemoryType};
    use cosmwasm_std::testing::{MockStorage, MockQuerier, MockApi};

    fn setup_test_env() -> Result<(Store<HostEnv<MockStorage, MockApi, MockQuerier>>, Memory), ExecutorError> {
        let engine = Engine::default();
        let mut host_env = HostEnv::new(
            MockStorage::new(),
            MockApi::default(),
            MockQuerier::new(&[]),
            1_000_000,
        );

        let mut store = Store::new(&engine, host_env);
        
        // Create a memory with 1 page (64KB)
        let memory_type = MemoryType::new(1, Some(2));  // Min 1 page, max 2 pages
        let memory = Memory::new(&mut store, memory_type)
            .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;

        Ok((store, memory))
    }

    #[test]
    fn test_memory_operations() -> Result<(), ExecutorError> {
        let (mut store, memory) = setup_test_env()?;
        store.data_mut().set_memory(memory);
        
        // Write small test data
        let test_data = b"test";
        let (data_ptr, data_len) = write_memory(&mut store, test_data)?;
        
        // Read and verify the data
        let read_data = read_memory(&mut store, data_ptr, data_len)?;
        assert_eq!(read_data, test_data);
        
        // Test writing larger data
        let large_data = vec![1u8; 1000];
        let (large_ptr, large_len) = write_memory(&mut store, &large_data)?;
        
        // Read and verify larger data
        let read_large = read_memory(&mut store, large_ptr, large_len)?;
        assert_eq!(read_large, large_data);
        
        Ok(())
    }
}
