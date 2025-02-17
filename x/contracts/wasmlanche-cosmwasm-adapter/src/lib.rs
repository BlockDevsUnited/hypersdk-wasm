use cosmwasm_std::{Api, Querier, Storage, MessageInfo, QueryRequest, ContractResult, Binary};
use wasmtime::Engine;
use serde::Serialize;

pub mod crypto;
pub mod error;
pub mod host;
pub mod executor;
pub mod imports;

use crate::executor::Executor;
use crate::error::ExecutorError;

pub struct WasmAdapter<S, A, Q>
where
    S: Storage + Clone + 'static,
    A: Api + Clone + 'static,
    Q: Querier + Clone + 'static,
{
    executor: Option<Executor<S, A, Q>>,
    storage: S,
    api: A,
    querier: Q,
    gas_limit: u64,
    engine: Engine,
}

impl<S, A, Q> WasmAdapter<S, A, Q>
where
    S: Storage + Clone + 'static,
    A: Api + Clone + 'static,
    Q: Querier + Clone + 'static,
{
    pub fn new(storage: S, api: A, querier: Q, gas_limit: u64) -> Self {
        Self {
            executor: None,
            storage,
            api,
            querier,
            gas_limit,
            engine: Engine::default(),
        }
    }

    pub fn store_code(&mut self, code: &[u8]) -> Result<(), ExecutorError> {
        let module = wasmtime::Module::new(&self.engine, code)
            .map_err(|e| ExecutorError::RuntimeError(e.to_string()))?;

        self.executor = Some(Executor::new(
            self.storage.clone(),
            self.api.clone(),
            self.querier.clone(),
            self.gas_limit,
            self.engine.clone(),
            module,
        ));

        Ok(())
    }

    pub fn instantiate(&mut self, msg: &[u8], info: MessageInfo, gas_limit: Option<u64>) -> Result<Vec<u8>, ExecutorError> {
        self.executor
            .as_mut()
            .ok_or_else(|| ExecutorError::NoContract)?
            .instantiate(msg, &info, gas_limit)
    }

    pub fn execute(&mut self, msg: &[u8], info: MessageInfo, gas_limit: Option<u64>) -> Result<Vec<u8>, ExecutorError> {
        self.executor
            .as_mut()
            .ok_or_else(|| ExecutorError::NoContract)?
            .execute(msg, &info, gas_limit)
    }

    pub fn query<C: Serialize>(&mut self, query: &QueryRequest<C>) -> Result<ContractResult<Binary>, ExecutorError> {
        self.executor
            .as_mut()
            .ok_or_else(|| ExecutorError::NoContract)?
            .query(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{testing::{MockApi, MockQuerier}, Addr, MessageInfo, Empty};
    use std::sync::{Arc, RwLock};
    use std::collections::HashMap;

    #[derive(Clone)]
    struct ClonableStorage {
        inner: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>
    }

    impl ClonableStorage {
        fn new() -> Self {
            Self {
                inner: Arc::new(RwLock::new(HashMap::new()))
            }
        }
    }

    impl Storage for ClonableStorage {
        fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
            self.inner.read().unwrap().get(key).cloned()
        }

        fn set(&mut self, key: &[u8], value: &[u8]) {
            self.inner.write().unwrap().insert(key.to_vec(), value.to_vec());
        }

        fn remove(&mut self, key: &[u8]) {
            self.inner.write().unwrap().remove(key);
        }

        fn range<'a>(&'a self, _start: Option<&[u8]>, _end: Option<&[u8]>, _order: cosmwasm_std::Order) -> Box<dyn Iterator<Item = cosmwasm_std::Record> + 'a> {
            Box::new(std::iter::empty())
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
            self.inner.read().unwrap().raw_query(bin_request)
        }
    }

    #[test]
    fn test_instantiate() {
        let engine = Engine::default();
        let wasm = wat::parse_str(r#"
            (module
                (type $t0 (func (param i32 i32 i32) (result i32)))
                (type $t1 (func (param i32)))
                (type $t2 (func (param i32) (result i32)))
                
                (import "env" "db_read" (func $db_read (param i32) (result i32)))
                (import "env" "db_write" (func $db_write (param i32 i32)))
                (import "env" "db_remove" (func $db_remove (param i32)))
                (import "env" "debug" (func $debug (param i32)))
                (import "env" "abort" (func $abort (param i32)))
                
                (memory $memory (export "memory") 1)
                (func $allocate (export "allocate") (param i32) (result i32) (local.get 0))
                (func $deallocate (export "deallocate") (param i32))
                (func $instantiate (export "instantiate") (param i32 i32 i32) (result i32) (i32.const 0))
                (func $execute (export "execute") (param i32 i32 i32) (result i32) (i32.const 0))
                (func $query (export "query") (param i32 i32 i64) (result i32) (i32.const 0))
            )
        "#).unwrap();

        let module = wasmtime::Module::new(&engine, &wasm).unwrap();
        
        let mut adapter = WasmAdapter::new(
            ClonableStorage::new(),
            MockApi::default(),
            ClonableQuerier::new(),
            1_000_000,
        );

        adapter.store_code(&wasm).unwrap();

        let msg = b"{}";
        let info = MessageInfo {
            sender: Addr::unchecked("sender"),
            funds: vec![],
        };

        let result = adapter.instantiate(msg, info, None);
        assert!(result.is_ok());
    }
}
