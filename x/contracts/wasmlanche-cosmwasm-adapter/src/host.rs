use std::cell::RefCell;
use wasmtime::{Memory, Store, AsContextMut};
use cosmwasm_std::{Storage, Api, Querier, CanonicalAddr};
use crate::error::ExecutorError;
use std::collections::HashMap;

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
    allocated_regions: RefCell<HashMap<u32, u32>>, // Maps ptr -> size
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
            allocated_regions: RefCell::new(HashMap::new()),
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

        // Add 4 bytes for the length prefix that CosmWasm expects
        let total_size = size.checked_add(4)
            .ok_or_else(|| anyhow::anyhow!("Memory size overflow"))?;

        *next_ptr = next_ptr.checked_add(total_size)
            .ok_or_else(|| anyhow::anyhow!("Memory size overflow"))?;

        // Ensure memory alignment (align to 8 bytes)
        *next_ptr = (*next_ptr + 7) & !7;

        // Track this allocation
        self.allocated_regions.borrow_mut().insert(ptr, total_size);

        Ok(ptr)
    }

    pub fn deallocate(&mut self, ptr: u32) -> anyhow::Result<()> {
        // Check if this pointer was allocated
        let mut regions = self.allocated_regions.borrow_mut();
        if let Some(_) = regions.remove(&ptr) {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Attempted to deallocate unallocated pointer: {}", ptr))
        }
    }

    pub fn addr_validate(&self, addr: &str) -> Result<(), ExecutorError> {
        self.api.addr_validate(addr)
            .map_err(|e| ExecutorError::ApiError(e.to_string()))?;
        Ok(())
    }

    pub fn addr_canonicalize(&self, human: &str) -> Result<CanonicalAddr, ExecutorError> {
        self.api.addr_canonicalize(human)
            .map_err(|e| ExecutorError::ApiError(e.to_string()))
    }

    pub fn addr_humanize(&self, canonical: &[u8]) -> Result<String, ExecutorError> {
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
    // Get all the data we need from the store first
    let (ptr, memory) = {
        let env = store.data();
        let ptr = env.next_ptr.borrow().clone() as usize;
        let memory = env.memory.as_ref()
            .ok_or_else(|| ExecutorError::NoMemoryExport)?
            .clone();
        (ptr, memory)
    };
    let len = data.len();

    // Check if we have enough memory
    let total_size = len + 4;
    let current_pages = memory.size(&mut store.as_context_mut());
    let required_pages = (total_size as u64 + 65535) / 65536;
    
    if current_pages < required_pages {
        // Try to grow memory
        memory.grow(&mut store.as_context_mut(), required_pages - current_pages)
            .map_err(|e| ExecutorError::MemoryAccessError(format!("Failed to grow memory: {}", e)))?;
    }

    // Write length prefix (4 bytes)
    let len_bytes = (len as u32).to_le_bytes();
    memory.write(store.as_context_mut(), ptr, &len_bytes)
        .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;

    // Write data
    memory.write(store.as_context_mut(), ptr + 4, data)
        .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;

    // Update next_ptr
    let total_size = (len + 4) as u32;
    let mut next_ptr = store.data().next_ptr.borrow_mut();
    *next_ptr += total_size;
    // Ensure memory alignment (align to 8 bytes)
    *next_ptr = (*next_ptr + 7) & !7;

    Ok((ptr, len))
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
    // Get the memory reference first
    let memory = store.data().memory.as_ref()
        .ok_or_else(|| ExecutorError::NoMemoryExport)?
        .clone();

    // Check if we can read the length prefix
    let memory_size = memory.size(&mut store.as_context_mut()) * 65536;
    if (ptr as u64) + 4 > memory_size {
        return Err(ExecutorError::MemoryAccessError("Cannot read length prefix".to_string()));
    }

    // Read length prefix (4 bytes)
    let mut len_bytes = [0u8; 4];
    memory.read(store.as_context_mut(), ptr, &mut len_bytes)
        .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    if len > max_length {
        return Err(ExecutorError::MemoryAccessError("Length exceeds maximum".to_string()));
    }

    // Check if we can read the data
    if (ptr as u64) + 4 + (len as u64) > memory_size {
        return Err(ExecutorError::MemoryAccessError("Cannot read data".to_string()));
    }

    // Read data
    let mut data = vec![0u8; len];
    memory.read(store.as_context_mut(), ptr + 4, &mut data)
        .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::MemoryType;
    use cosmwasm_std::{Addr, Binary, Order, Record, StdResult, SystemResult, ContractResult, QuerierResult};
    use cosmwasm_std::{VerificationError, RecoverPubkeyError};

    #[derive(Default, Clone)]
    struct MockStorage {
        data: HashMap<Vec<u8>, Vec<u8>>,
    }

    impl Storage for MockStorage {
        fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
            self.data.get(key).cloned()
        }

        fn set(&mut self, key: &[u8], value: &[u8]) {
            self.data.insert(key.to_vec(), value.to_vec());
        }

        fn remove(&mut self, key: &[u8]) {
            self.data.remove(key);
        }

        fn range<'a>(&'a self, _start: Option<&[u8]>, _end: Option<&[u8]>, _order: Order) -> Box<dyn Iterator<Item = Record> + 'a> {
            Box::new(std::iter::empty())
        }
    }

    #[derive(Default, Clone)]
    struct MockApi;

    impl Api for MockApi {
        fn addr_validate(&self, human: &str) -> StdResult<Addr> {
            Ok(Addr::unchecked(human))
        }

        fn addr_canonicalize(&self, human: &str) -> StdResult<CanonicalAddr> {
            Ok(CanonicalAddr::from(human.as_bytes()))
        }

        fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
            Ok(Addr::unchecked(String::from_utf8_lossy(canonical.as_slice())))
        }

        fn secp256k1_verify(&self, _message_hash: &[u8], _signature: &[u8], _public_key: &[u8]) -> Result<bool, VerificationError> {
            Ok(true)
        }

        fn secp256k1_recover_pubkey(&self, _message_hash: &[u8], _signature: &[u8], _recovery_param: u8) -> Result<Vec<u8>, RecoverPubkeyError> {
            Ok(vec![0u8; 65])
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

    #[derive(Default, Clone)]
    struct MockQuerier;

    impl Querier for MockQuerier {
        fn raw_query(&self, _bin_request: &[u8]) -> QuerierResult {
            SystemResult::Ok(ContractResult::Ok(Binary::from(vec![])))
        }
    }

    fn setup_test_env() -> Result<(Store<HostEnv<MockStorage, MockApi, MockQuerier>>, Memory), ExecutorError> {
        let engine = wasmtime::Engine::default();
        let mut store = Store::new(
            &engine,
            HostEnv::new(
                MockStorage::default(),
                MockApi::default(),
                MockQuerier::default(),
                1_000_000,
            ),
        );

        // Create a memory with 2 pages (128KB) and allow it to grow up to 10 pages
        let memory_type = MemoryType::new(2, Some(10));
        let memory = Memory::new(&mut store, memory_type)
            .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;

        // Initialize the memory with zeros
        let zero_page = vec![0u8; 65536];
        memory.write(&mut store.as_context_mut(), 0, &zero_page)
            .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;
        memory.write(&mut store.as_context_mut(), 65536, &zero_page)
            .map_err(|e| ExecutorError::MemoryAccessError(e.to_string()))?;

        store.data_mut().set_memory(memory.clone());
        Ok((store, memory))
    }

    #[test]
    fn test_memory_operations() -> Result<(), ExecutorError> {
        let (mut store, _) = setup_test_env()?;
        
        // Test small data
        let test_data = b"Hello, World!";
        let (ptr, len) = write_memory(&mut store, test_data)?;
        assert_eq!(len, test_data.len());
        let read_data = read_memory(&mut store, ptr, 1024)?;
        assert_eq!(read_data, test_data);

        // Test larger data
        let large_data = vec![42u8; 1000];
        let (ptr2, len2) = write_memory(&mut store, &large_data)?;
        assert_eq!(len2, large_data.len());
        let read_data2 = read_memory(&mut store, ptr2, 2000)?;
        assert_eq!(read_data2, large_data);

        // Test reading with too small max_length
        let result = read_memory(&mut store, ptr2, 500);
        assert!(result.is_err());

        Ok(())
    }
}
