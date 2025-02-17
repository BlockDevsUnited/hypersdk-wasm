use cosmwasm_std::{
    Storage, Api, Querier, MessageInfo, QueryRequest, ContractResult, Binary,
    Coin, StdResult,
};
use wasmtime::Engine;
use serde::Serialize;

mod crypto;
mod error;
mod executor;
mod host;
mod imports;
mod testing;

pub use crypto::CryptoApi;
pub use error::ExecutorError;
pub use executor::WasmExecutor;
pub use testing::{ThreadSafeApi, ThreadSafeStorage, ThreadSafeQuerier};

pub struct WasmAdapter<S, A, Q>
where
    S: Storage + Clone + 'static,
    A: Api + Clone + 'static,
    Q: Querier + Clone + 'static,
{
    storage: S,
    api: A,
    querier: Q,
    executor: Option<WasmExecutor<S, A, Q>>,
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
            storage,
            api,
            querier,
            executor: None,
            gas_limit,
            engine: Engine::default(),
        }
    }

    pub fn instantiate(
        &mut self,
        wasm: &[u8],
        msg: &[u8],
        info: &MessageInfo,
        _funds: Vec<Coin>,
    ) -> Result<Vec<u8>, ExecutorError> {
        let module = wasmtime::Module::new(&self.engine, wasm)?;
        
        let mut executor = WasmExecutor::new(
            self.storage.clone(),
            self.api.clone(),
            self.querier.clone(),
            self.gas_limit,
            self.engine.clone(),
            module,
        )?;

        let result = executor.instantiate(msg, info, Some(self.gas_limit))?;
        self.executor = Some(executor);
        Ok(result)
    }

    pub fn execute(
        &mut self,
        msg: &[u8],
        info: &MessageInfo,
    ) -> Result<Vec<u8>, ExecutorError> {
        if let Some(executor) = &mut self.executor {
            executor.execute(msg, info, Some(self.gas_limit))
        } else {
            Err(ExecutorError::NotInstantiated)
        }
    }

    pub fn query<C: Serialize + Clone + std::fmt::Debug + PartialEq>(
        &mut self,
        request: &QueryRequest<C>,
    ) -> StdResult<ContractResult<Binary>> {
        if let Some(executor) = &mut self.executor {
            executor
                .query(request)
                .map_err(|e| cosmwasm_std::StdError::generic_err(e.to_string()))
        } else {
            Err(cosmwasm_std::StdError::generic_err("Contract not instantiated"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_info, MockApi};
    use testing::{ThreadSafeStorage, ThreadSafeQuerier};

    #[test]
    fn test_adapter() {
        let mut adapter = WasmAdapter::new(
            ThreadSafeStorage::new(),
            MockApi::default(),
            ThreadSafeQuerier::new(),
            1_000_000,
        );

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

        let result = adapter.instantiate(&wasm, b"hello", &mock_info("sender", &[]), vec![]);
        assert!(result.is_ok());
    }
}
