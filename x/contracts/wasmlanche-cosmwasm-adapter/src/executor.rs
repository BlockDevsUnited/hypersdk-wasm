use cosmwasm_std::{Storage, Api, Querier, MessageInfo, QueryRequest, ContractResult, Binary, to_json_binary, from_json, Addr};
use wasmtime::{Store, Module, Engine, Linker, Instance};
use anyhow::Result;
use serde::Serialize;

use crate::error::ExecutorError;
use crate::host::{self, HostEnv};
use crate::imports;
use crate::testing::{ThreadSafeStorage, ThreadSafeQuerier};

pub struct WasmExecutor<S, A, Q>
where
    S: Storage + Clone + 'static,
    A: Api + Clone + 'static,
    Q: Querier + Clone + 'static,
{
    store: Store<HostEnv<S, A, Q>>,
    instance: Instance,
    gas_limit: u64,
    module: Module,
    linker: Linker<HostEnv<S, A, Q>>,
}

impl<S, A, Q> WasmExecutor<S, A, Q>
where
    S: Storage + Clone + 'static,
    A: Api + Clone + 'static,
    Q: Querier + Clone + 'static,
{
    pub fn new(
        storage: S,
        api: A,
        querier: Q,
        gas_limit: u64,
        engine: Engine,
        module: Module,
    ) -> Result<Self, ExecutorError> {
        let mut store = Store::new(
            &engine,
            HostEnv::new(storage, api, querier, gas_limit),
        );

        let mut linker = Linker::new(&engine);
        imports::define_imports(&mut linker, &mut store, module.clone())?;
        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| ExecutorError::InstantiationError(e.to_string()))?;

        // Get memory from instance and set it in the host environment
        if let Some(memory) = instance.get_export(&mut store, "memory").and_then(|e| e.into_memory()) {
            store.data_mut().set_memory(memory);
        }

        Ok(Self {
            store,
            instance,
            gas_limit,
            module,
            linker,
        })
    }

    pub fn set_gas_limit(&mut self, gas_limit: u64) {
        self.gas_limit = gas_limit;
        self.store.data_mut().set_gas_limit(gas_limit);
    }

    pub fn instantiate(
        &mut self,
        msg: &[u8],
        info: &MessageInfo,
        gas_limit: Option<u64>,
    ) -> Result<Vec<u8>, ExecutorError> {
        if let Some(gas) = gas_limit {
            self.set_gas_limit(gas);
        }

        let instantiate = self.instance
            .get_typed_func::<(i32, i32, i32), i32>(&mut self.store, "instantiate")
            .map_err(|e| ExecutorError::RuntimeError(format!("Failed to get instantiate function: {}", e)))?;

        let (msg_ptr, msg_len) = host::write_memory(&mut self.store, msg)?;
        let (info_ptr, info_len) = host::write_memory(&mut self.store, &to_json_binary(info).unwrap())?;
        let (gas_info_ptr, _gas_info_len) = host::write_memory(&mut self.store, &[0u8; 4])?;

        let result_ptr = instantiate
            .call(&mut self.store, (msg_ptr as i32, info_ptr as i32, gas_info_ptr as i32))
            .map_err(|e| ExecutorError::RuntimeError(e.to_string()))?;

        host::read_memory(&mut self.store, result_ptr as usize, msg_len.max(info_len))
    }

    pub fn execute(
        &mut self,
        msg: &[u8],
        info: &MessageInfo,
        gas_limit: Option<u64>,
    ) -> Result<Vec<u8>, ExecutorError> {
        if let Some(gas) = gas_limit {
            self.set_gas_limit(gas);
        }

        let execute = self.instance
            .get_typed_func::<(i32, i32, i32), i32>(&mut self.store, "execute")
            .map_err(|e| ExecutorError::RuntimeError(format!("Failed to get execute function: {}", e)))?;

        let (msg_ptr, msg_len) = host::write_memory(&mut self.store, msg)?;
        let (info_ptr, info_len) = host::write_memory(&mut self.store, &to_json_binary(info).unwrap())?;
        let (gas_info_ptr, _gas_info_len) = host::write_memory(&mut self.store, &[0u8; 4])?;

        let result_ptr = execute
            .call(&mut self.store, (msg_ptr as i32, info_ptr as i32, gas_info_ptr as i32))
            .map_err(|e| ExecutorError::RuntimeError(e.to_string()))?;

        host::read_memory(&mut self.store, result_ptr as usize, msg_len.max(info_len))
    }

    pub fn query<C: Serialize>(
        &mut self,
        query: &QueryRequest<C>,
    ) -> Result<ContractResult<Binary>, ExecutorError> {
        let query_func = self.instance
            .get_typed_func::<(i32, i32, i64), i32>(&mut self.store, "query")
            .map_err(|e| ExecutorError::RuntimeError(format!("Failed to get query function: {}", e)))?;

        let query_msg = to_json_binary(query).unwrap();
        let (query_ptr, query_len) = host::write_memory(&mut self.store, &query_msg)?;
        let (gas_info_ptr, _gas_info_len) = host::write_memory(&mut self.store, &[0u8; 4])?;

        let result_ptr = query_func
            .call(&mut self.store, (query_ptr as i32, gas_info_ptr as i32, self.gas_limit as i64))
            .map_err(|e| ExecutorError::RuntimeError(e.to_string()))?;

        let result_data = host::read_memory(&mut self.store, result_ptr as usize, query_len)?;
        
        from_json(&result_data)
            .map_err(|e| ExecutorError::RuntimeError(format!("Failed to deserialize response: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{MockApi, MockQuerier};
    use cosmwasm_std::{MemoryStorage, Order, Record, Addr};
    use std::sync::{Arc, RwLock};

    #[derive(Clone)]
    struct ClonableStorage {
        inner: Arc<RwLock<MemoryStorage>>
    }
    
    impl ClonableStorage {
        fn new() -> Self {
            Self {
                inner: Arc::new(RwLock::new(MemoryStorage::default()))
            }
        }
    }
    
    impl Storage for ClonableStorage {
        fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
            self.inner.read().unwrap().get(key)
        }

        fn set(&mut self, key: &[u8], value: &[u8]) {
            self.inner.write().unwrap().set(key, value)
        }

        fn remove(&mut self, key: &[u8]) {
            self.inner.write().unwrap().remove(key)
        }

        fn range<'a>(&'a self, start: Option<&[u8]>, end: Option<&[u8]>, order: Order) -> Box<dyn Iterator<Item = Record> + 'a> {
            let inner = self.inner.read().unwrap();
            let range = inner.range(start, end, order);
            let vec: Vec<_> = range.collect();
            Box::new(vec.into_iter())
        }
    }

    #[derive(Clone)]
    struct ClonableQuerier {
        inner: Arc<RwLock<MockQuerier>>
    }

    impl ClonableQuerier {
        fn new() -> Self {
            Self {
                inner: Arc::new(RwLock::new(MockQuerier::new(&[])))
            }
        }
    }

    impl Querier for ClonableQuerier {
        fn raw_query(&self, bin_request: &[u8]) -> cosmwasm_std::QuerierResult {
            let inner = self.inner.read().unwrap();
            inner.raw_query(bin_request)
        }
    }

    #[test]
    fn test_executor() {
        let engine = Engine::default();
        let wasm = wat::parse_str(r#"
            (module
                (type $t0 (func (param i32) (result i32)))
                (type $t1 (func (param i32 i32)))
                (type $t2 (func (param i32)))
                (type $t3 (func (param i32 i32) (result i32)))
                (type $t4 (func (param i32 i32 i32) (result i32)))
                (type $t5 (func (param i32 i32 i32) (result i64)))
                
                ;; Memory management imports
                (import "env" "allocate" (func $allocate (type $t0)))
                (import "env" "deallocate" (func $deallocate (type $t2)))
                
                ;; Required contract functions
                (func $instantiate (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
                    (local $result i32)
                    (local.set $result (call $allocate (i32.const 4)))
                    (i32.store (local.get $result) (i32.const 0))
                    (local.get $result))
                
                (func $execute (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
                    (local $result i32)
                    (local.set $result (call $allocate (i32.const 4)))
                    (i32.store (local.get $result) (i32.const 0))
                    (local.get $result))
                
                (func $query (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
                    (local $result i32)
                    (local.set $result (call $allocate (i32.const 4)))
                    (i32.store (local.get $result) (i32.const 0))
                    (local.get $result))
                
                (memory $memory 2 10)
                (export "memory" (memory $memory))
                (export "instantiate" (func $instantiate))
                (export "execute" (func $execute))
                (export "query" (func $query))
            )
        "#).unwrap();

        let module = Module::new(&engine, wasm).unwrap();
        let mut executor = WasmExecutor::new(
            ThreadSafeStorage::new(),
            MockApi::default(),
            ThreadSafeQuerier::new(),
            1_000_000,
            engine,
            module,
        ).unwrap();

        let info = MessageInfo {
            sender: Addr::unchecked("sender"),
            funds: vec![],
        };

        let result = executor.instantiate(b"hello", &info, None);
        assert!(result.is_ok());
    }
}
