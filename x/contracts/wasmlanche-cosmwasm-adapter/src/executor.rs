use std::sync::{Arc, Mutex};
use cosmwasm_std::{Api, Querier, Storage};
use wasmtime::{Engine, Instance, Linker, Module, Store, Caller};
use serde::Serialize;
use std::future::Future;

use crate::error::ExecutorError;
use crate::host::HostEnv;

pub struct Executor<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    engine: Engine,
    store: Store<Arc<Mutex<HostEnv<S, A, Q>>>>,
    linker: Linker<Arc<Mutex<HostEnv<S, A, Q>>>>,
    instance: Option<Instance>,
    gas_limit: u64,
}

impl<S, A, Q> Executor<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    pub fn new(storage: S, api: A, querier: Q, gas_limit: u64) -> Result<Self, ExecutorError> {
        let engine = Engine::default();
        let env = Arc::new(Mutex::new(HostEnv::new(storage, api, querier, gas_limit)));
        let store = Store::new(&engine, env);
        let linker = Linker::new(&engine);
        
        let mut executor = Self {
            engine,
            store,
            linker,
            instance: None,
            gas_limit,
        };
        
        executor.define_functions()?;
        Ok(executor)
    }

    fn define_functions(&mut self) -> Result<(), ExecutorError> {
        let env = Arc::clone(&self.store.data());
        
        self.linker.func_wrap1_async("env", "allocate", {
            let env = Arc::clone(&env);
            move |_caller: Caller<'_, Arc<Mutex<HostEnv<S, A, Q>>>>, size: i32| -> Box<dyn Future<Output = i32> + Send> {
                let size = size as usize;
                let env = Arc::clone(&env);
                Box::new(async move {
                    match env.lock() {
                        Ok(mut guard) => {
                            match guard.allocate(size) {
                                Ok(ptr) => ptr as i32,
                                Err(_) => -1,
                            }
                        }
                        Err(_) => -1,
                    }
                })
            }
        })?;

        self.linker.func_wrap1_async("env", "deallocate", {
            let env = Arc::clone(&env);
            move |_caller: Caller<'_, Arc<Mutex<HostEnv<S, A, Q>>>>, ptr: i32| -> Box<dyn Future<Output = i32> + Send> {
                let ptr = ptr as usize;
                let env = Arc::clone(&env);
                Box::new(async move {
                    if let Ok(mut guard) = env.lock() {
                        let _ = guard.deallocate(ptr);
                    }
                    0
                })
            }
        })?;

        self.linker.func_wrap1_async("env", "db_read", {
            let env = Arc::clone(&env);
            move |_caller: Caller<'_, Arc<Mutex<HostEnv<S, A, Q>>>>, key_ptr: i32| -> Box<dyn Future<Output = i32> + Send> {
                let key_ptr = key_ptr as usize;
                let env = Arc::clone(&env);
                Box::new(async move {
                    let guard = match env.lock() {
                        Ok(guard) => guard,
                        Err(_) => return -1,
                    };

                    let key = match guard.ptr_to_slice(key_ptr) {
                        Ok(key) => key,
                        Err(_) => return -1,
                    };

                    let storage = match guard.storage() {
                        Ok(storage) => storage,
                        Err(_) => return -1,
                    };

                    let value = match storage.get(key) {
                        Some(value) => value,
                        None => return -1,
                    };

                    // Drop everything before taking a new lock
                    drop(storage);
                    drop(guard);

                    let mut new_guard = match env.lock() {
                        Ok(guard) => guard,
                        Err(_) => return -1,
                    };

                    match new_guard.allocate(value.len()) {
                        Ok(ptr) => {
                            let _ = new_guard.write_memory(ptr, &value);
                            ptr as i32
                        }
                        Err(_) => -1,
                    }
                })
            }
        })?;

        self.linker.func_wrap2_async("env", "db_write", {
            let env = Arc::clone(&env);
            move |_caller: Caller<'_, Arc<Mutex<HostEnv<S, A, Q>>>>, key_ptr: i32, value_ptr: i32| -> Box<dyn Future<Output = i32> + Send> {
                let key_ptr = key_ptr as usize;
                let value_ptr = value_ptr as usize;
                let env = Arc::clone(&env);
                Box::new(async move {
                    if let Ok(guard) = env.lock() {
                        if let (Ok(key), Ok(value)) = (guard.ptr_to_slice(key_ptr), guard.ptr_to_slice(value_ptr)) {
                            if let Ok(mut storage) = guard.storage_mut() {
                                storage.set(key, value);
                                return 0;
                            }
                        }
                    }
                    -1
                })
            }
        })?;

        self.linker.func_wrap1_async("env", "db_remove", {
            let env = Arc::clone(&env);
            move |_caller: Caller<'_, Arc<Mutex<HostEnv<S, A, Q>>>>, key_ptr: i32| -> Box<dyn Future<Output = i32> + Send> {
                let key_ptr = key_ptr as usize;
                let env = Arc::clone(&env);
                Box::new(async move {
                    if let Ok(guard) = env.lock() {
                        if let Ok(key) = guard.ptr_to_slice(key_ptr) {
                            if let Ok(mut storage) = guard.storage_mut() {
                                storage.remove(key);
                                return 0;
                            }
                        }
                    }
                    -1
                })
            }
        })?;

        self.linker.func_wrap1_async("env", "db_scan", {
            let env = Arc::clone(&env);
            move |_caller: Caller<'_, Arc<Mutex<HostEnv<S, A, Q>>>>, start_ptr: i32| -> Box<dyn Future<Output = i32> + Send> {
                let request_ptr = start_ptr as usize;
                let env = Arc::clone(&env);
                Box::new(async move {
                    match env.lock() {
                        Ok(guard) => {
                            match guard.ptr_to_slice(request_ptr) {
                                Ok(_request) => {
                                    // TODO: Implement proper scanning
                                    -1
                                }
                                Err(_) => -1,
                            }
                        }
                        Err(_) => -1,
                    }
                })
            }
        })?;

        Ok(())
    }

    pub fn instantiate(&mut self, wasm_bytes: &[u8]) -> Result<(), ExecutorError> {
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| ExecutorError::Instantiation(e.to_string()))?;
        
        let instance = self.linker
            .instantiate(&mut self.store, &module)
            .map_err(|e| ExecutorError::Instantiation(e.to_string()))?;
        
        self.instance = Some(instance);
        Ok(())
    }

    pub fn get_exports(&mut self) -> Result<(Vec<u8>, Vec<u8>), ExecutorError> {
        let instance = self.instance.as_ref()
            .ok_or_else(|| ExecutorError::InstanceNotFound)?;
        
        let instantiate = if instance.get_export(&mut self.store, "instantiate").is_some() {
            "instantiate".as_bytes().to_vec()
        } else {
            Vec::new()
        };
        
        let execute = if instance.get_export(&mut self.store, "execute").is_some() {
            "execute".as_bytes().to_vec()
        } else {
            Vec::new()
        };
        
        Ok((instantiate, execute))
    }

    pub fn instantiate_contract<T: Serialize>(
        &mut self,
        code: &[u8],
        msg: T,
        label: String,
    ) -> Result<Vec<u8>, ExecutorError> {
        self.instantiate(code)?;
        
        let msg_bytes = serde_json::to_vec(&msg)
            .map_err(|e| ExecutorError::Serialization(e.to_string()))?;

        // Get the instance before any store operations
        let instance = self.instance.as_ref()
            .ok_or_else(|| ExecutorError::InstanceNotFound)?;

        // Store the code and label
        {
            let env = self.store.data();
            let guard = env.lock()
                .map_err(|_| ExecutorError::Execution("Failed to lock environment".to_string()))?;

            guard.storage_mut()
                .map_err(|_| ExecutorError::Execution("Failed to access storage".to_string()))?
                .set(code, label.as_bytes());
        } // guard is dropped here

        let instantiate = instance.get_typed_func::<(i32, i32, i32), i32>(&mut self.store, "instantiate")
            .map_err(|e| ExecutorError::EntryPoint(format!("Failed to get instantiate function: {}", e)))?;

        let result = instantiate.call(&mut self.store, (0, 0, 0))
            .map_err(|e| ExecutorError::Execution(format!("Failed to call instantiate: {}", e)))?;
        
        if result < 0 {
            return Err(ExecutorError::Instantiation("Contract instantiation failed".to_string()));
        }
        
        Ok(msg_bytes)
    }

    pub fn execute_contract<T: Serialize>(
        &mut self,
        contract_addr: &str,
        msg: T,
    ) -> Result<Vec<u8>, ExecutorError> {
        let msg_bytes = serde_json::to_vec(&msg)
            .map_err(|e| ExecutorError::Serialization(e.to_string()))?;
        
        // Verify contract exists
        let env = self.store.data();
        let exists = env.lock().map_err(|_| ExecutorError::Execution("Failed to lock environment".to_string())).and_then(|guard| {
            guard.storage().map_err(|_| ExecutorError::Execution("Failed to access storage".to_string())).and_then(|storage| {
                Ok(storage.get(contract_addr.as_bytes()).is_some())
            })
        })?;

        if !exists {
            return Err(ExecutorError::Execution(format!("Contract not found at address: {}", contract_addr)));
        }

        // Call the execute export
        let execute = self.instance.as_ref().ok_or(ExecutorError::Execution("No instance available".to_string()))?.get_typed_func::<(i32, i32), i32>(&mut self.store, "execute")?;
        let result = execute.call(&mut self.store, (0, 0))?;
        
        if result < 0 {
            return Err(ExecutorError::Execution("Contract execution failed".to_string()));
        }
        
        Ok(msg_bytes)
    }

    pub fn query_contract<T: Serialize>(
        &mut self,
        contract_addr: &str,
        msg: T,
    ) -> Result<Vec<u8>, ExecutorError> {
        let msg_bytes = serde_json::to_vec(&msg)
            .map_err(|e| ExecutorError::Serialization(e.to_string()))?;
        
        // Verify contract exists
        let env = self.store.data();
        let exists = env.lock().map_err(|_| ExecutorError::Execution("Failed to lock environment".to_string())).and_then(|guard| {
            guard.storage().map_err(|_| ExecutorError::Execution("Failed to access storage".to_string())).and_then(|storage| {
                Ok(storage.get(contract_addr.as_bytes()).is_some())
            })
        })?;

        if !exists {
            return Err(ExecutorError::EntryPoint(format!("Contract not found at address: {}", contract_addr)));
        }

        // Call the query export
        let query = self.instance.as_ref().ok_or(ExecutorError::EntryPoint("No instance available".to_string()))?.get_typed_func::<(i32, i32), i32>(&mut self.store, "query")?;
        let result = query.call(&mut self.store, (0, 0))?;
        
        if result < 0 {
            return Err(ExecutorError::EntryPoint("Query failed".to_string()));
        }
        
        Ok(msg_bytes)
    }

    pub fn get_gas_limit(&self) -> u64 {
        self.gas_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Binary, SystemResult, ContractResult};
    use cosmwasm_std::testing::MockApi;
    use serde_json::json;
    use std::fs;

    // Custom MockQuerier that implements Send + Sync + Clone
    #[derive(Default, Clone)]
    struct TestMockQuerier;

    impl cosmwasm_std::Querier for TestMockQuerier {
        fn raw_query(&self, _bin_request: &[u8]) -> SystemResult<ContractResult<Binary>> {
            SystemResult::Ok(ContractResult::Ok(Binary::default()))
        }
    }

    // Custom storage that implements Clone
    #[derive(Default, Clone)]
    struct TestStorage {
        data: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    }

    impl TestStorage {
        pub fn new() -> Self {
            Self {
                data: std::collections::HashMap::new(),
            }
        }
    }

    impl cosmwasm_std::Storage for TestStorage {
        fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
            self.data.get(key).cloned()
        }

        fn set(&mut self, key: &[u8], value: &[u8]) {
            self.data.insert(key.to_vec(), value.to_vec());
        }

        fn remove(&mut self, key: &[u8]) {
            self.data.remove(&key.to_vec());
        }

        fn range<'a>(
            &'a self,
            start: Option<&[u8]>,
            end: Option<&[u8]>,
            order: cosmwasm_std::Order,
        ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
            let mut records = Vec::new();
            
            for (k, v) in self.data.iter() {
                if start.map_or(true, |s| k.as_slice() >= s) && 
                   end.map_or(true, |e| k.as_slice() < e) {
                    records.push((k.to_vec(), v.to_vec()));
                }
            }

            if order == cosmwasm_std::Order::Descending {
                records.reverse();
            }

            Box::new(records.into_iter())
        }
    }

    fn load_test_contract() -> Vec<u8> {
        fs::read("test_contracts/counter.wasm").expect("Failed to read test contract")
    }

    fn setup_executor() -> Executor<TestStorage, MockApi, TestMockQuerier> {
        let storage = TestStorage::new();
        let api = MockApi::default();
        let querier = TestMockQuerier::default();
        let gas_limit = 1_000_000;

        Executor::new(storage, api, querier, gas_limit)
            .expect("Failed to create executor")
    }

    #[test]
    fn test_instantiate_contract() {
        let mut executor = setup_executor();
        let code = load_test_contract();
        let msg = json!({
            "verifier": "verifier",
            "beneficiary": "beneficiary",
        });

        let result = executor.instantiate_contract(&code, msg, "test_contract".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_contract() {
        let mut executor = setup_executor();
        
        // First instantiate
        let init_msg = json!({
            "count": 0
        });
        executor.instantiate_contract(
            &load_test_contract(),
            init_msg,
            "counter".to_string(),
        ).expect("Failed to instantiate contract");

        // Then execute increment
        let exec_msg = json!({
            "increment": {}
        });
        let result = executor.execute_contract(
            "counter",
            exec_msg,
        );

        assert!(result.is_ok(), "Execution failed: {:?}", result);
    }

    #[test]
    fn test_query_contract() {
        let mut executor = setup_executor();
        
        // First instantiate
        let init_msg = json!({
            "count": 0
        });
        executor.instantiate_contract(
            &load_test_contract(),
            init_msg,
            "counter".to_string(),
        ).expect("Failed to instantiate contract");

        // Then execute increment
        let exec_msg = json!({
            "increment": {}
        });
        executor.execute_contract(
            "counter",
            exec_msg,
        ).expect("Failed to execute contract");

        // Finally query
        let query_msg = json!({
            "get_count": {}
        });
        let result = executor.query_contract(
            "counter",
            query_msg,
        );

        assert!(result.is_ok(), "Query failed: {:?}", result);
        let count = String::from_utf8(result.unwrap()).expect("Invalid UTF-8");
        assert_eq!(count, "1", "Counter should be 1 after increment");
    }

    #[test]
    fn test_gas_limit() {
        let wasm_bytes = load_test_contract();
        let storage = TestStorage::new();
        let api = MockApi::default();
        let querier = TestMockQuerier::default();
        let gas_limit = 1; // Very low gas limit

        let mut executor = Executor::new(storage, api, querier, gas_limit)
            .expect("Failed to create executor");

        let result = executor.instantiate(&wasm_bytes);
        assert!(result.is_err());
        
        // Verify gas limit is set correctly
        assert_eq!(executor.get_gas_limit(), gas_limit);
    }

    #[test]
    fn test_storage_operations() {
        let mut storage = TestStorage::new();
        
        // Test set and get
        let key = b"test_key";
        let value = b"test_value";
        storage.set(key, value);
        assert_eq!(storage.get(key), Some(value.to_vec()));
        
        // Test remove
        storage.remove(key);
        assert_eq!(storage.get(key), None);
        
        // Test range
        let pairs = vec![
            (b"a".to_vec(), b"1".to_vec()),
            (b"b".to_vec(), b"2".to_vec()),
            (b"c".to_vec(), b"3".to_vec()),
        ];
        
        for (k, v) in pairs.iter() {
            storage.set(k, v);
        }
        
        // Test ascending order
        let result: Vec<_> = storage.range(None, None, cosmwasm_std::Order::Ascending)
            .collect();
        assert_eq!(result, pairs);
        
        // Test descending order
        let result: Vec<_> = storage.range(None, None, cosmwasm_std::Order::Descending)
            .collect();
        assert_eq!(result, pairs.into_iter().rev().collect::<Vec<_>>());
        
        // Test range with bounds
        let result: Vec<_> = storage.range(
            Some(b"a"), 
            Some(b"c"), 
            cosmwasm_std::Order::Ascending
        ).collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, b"a");
        assert_eq!(result[1].0, b"b");
    }
}
