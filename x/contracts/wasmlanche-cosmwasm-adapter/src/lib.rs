pub mod crypto;
pub mod executor;
pub mod hash;
pub mod state;
pub mod host;
pub mod error;

use cosmwasm_std::{Api, Querier, Storage, Binary, Response};
use executor::Executor;
use error::ExecutorError;
use serde::{de::DeserializeOwned, Serialize};
use crate::state::{MockStorage, MockApi, MockQuerier};

pub struct CosmWasmAdapter<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    executor: Option<Executor<S, A, Q>>,
}

impl<S, A, Q> CosmWasmAdapter<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self { executor: None }
    }

    pub fn instantiate_contract(
        &mut self,
        code: &[u8],
        storage: S,
        api: A,
        querier: Q,
        gas_limit: u64,
    ) -> Result<(), ExecutorError> {
        let executor = Executor::new(storage, api, querier, gas_limit);
        let _result = executor.instantiate_contract(
            code,
            serde_json::json!({}),
            "contract".to_string(),
        )?;
        self.executor = Some(executor);
        Ok(())
    }

    pub fn execute<T: Serialize + DeserializeOwned + 'static>(
        &mut self,
        contract_addr: String,
        msg: T,
    ) -> Result<Binary, ExecutorError> {
        match &self.executor {
            Some(executor) => executor.execute(contract_addr, msg),
            None => Err(ExecutorError::Instantiation(
                "Contract not instantiated".to_string(),
            )),
        }
    }

    pub fn query<T: Serialize + DeserializeOwned + 'static>(
        &mut self,
        contract_addr: String,
        msg: T,
    ) -> Result<Binary, ExecutorError> {
        match &self.executor {
            Some(executor) => executor.query(contract_addr, msg),
            None => Err(ExecutorError::Instantiation(
                "Contract not instantiated".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs;

    fn load_test_contract() -> Vec<u8> {
        fs::read("src/test_contract/target/wasm32-unknown-unknown/release/test_contract.wasm")
            .expect("Failed to read test contract")
    }

    #[test]
    fn test_contract_instantiation() {
        let mut adapter = CosmWasmAdapter::new();
        let wasm_bytes = load_test_contract();
        let storage = MockStorage::default();
        let api = MockApi::default();
        let querier = MockQuerier::default();

        let result = adapter.instantiate_contract(&wasm_bytes, storage, api, querier, 1_000_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_contract_execution() {
        let mut adapter = CosmWasmAdapter::new();
        let wasm_bytes = load_test_contract();
        let storage = MockStorage::default();
        let api = MockApi::default();
        let querier = MockQuerier::default();

        adapter
            .instantiate_contract(&wasm_bytes, storage, api, querier, 1_000_000)
            .unwrap();

        #[derive(Serialize, Deserialize)]
        struct ExecuteMsg {
            value: i32,
        }

        let msg = ExecuteMsg { value: 42 };
        let result = adapter.execute("contract_addr".to_string(), msg).unwrap();
        
        // For now, just check that we get a Binary response
        // TODO: Update this test once Executor implementation is complete
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_contract_query() {
        let mut adapter = CosmWasmAdapter::new();
        let wasm_bytes = load_test_contract();
        let storage = MockStorage::default();
        let api = MockApi::default();
        let querier = MockQuerier::default();

        adapter
            .instantiate_contract(&wasm_bytes, storage, api, querier, 1_000_000)
            .unwrap();

        #[derive(Serialize, Deserialize)]
        struct QueryMsg {
            key: String,
        }

        let msg = QueryMsg { key: "test".to_string() };
        let result = adapter.query("contract_addr".to_string(), msg).unwrap();
        
        // For now, just check that we get a Binary response
        // TODO: Update this test once Executor implementation is complete
        assert_eq!(result.len(), 0);
    }
}
