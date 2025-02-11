use wasmtime::Engine;
use cosmwasm_std::{Storage, Api, Querier, Binary};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

use crate::host::HostEnv;
use crate::error::ExecutorError;

pub struct Executor<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    engine: Engine,
    host_env: Arc<HostEnv<S, A, Q>>,
}

impl<S, A, Q> Executor<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    pub fn new(storage: S, api: A, querier: Q, gas_limit: u64) -> Self {
        let engine = Engine::default();
        let host_env = Arc::new(HostEnv::new(storage, api, querier, gas_limit));
        Self { engine, host_env }
    }

    pub fn instantiate_contract<M>(
        &self,
        code: &[u8],
        msg: M,
        _label: String,
    ) -> Result<Binary, ExecutorError>
    where
        M: Serialize + DeserializeOwned + 'static,
    {
        let _msg_bytes = serde_json::to_vec(&msg)
            .map_err(|e| ExecutorError::Serialization(e.to_string()))?;
        let _code = code.to_vec();
        // TODO: Implement actual instantiation
        Ok(Binary::default())
    }

    pub fn execute<M>(
        &self,
        _contract_addr: String,
        msg: M,
    ) -> Result<Binary, ExecutorError>
    where
        M: Serialize + DeserializeOwned + 'static,
    {
        let _msg_bytes = serde_json::to_vec(&msg)
            .map_err(|e| ExecutorError::Serialization(e.to_string()))?;
        // TODO: Implement actual execution
        Ok(Binary::default())
    }

    pub fn query<M>(
        &self,
        _contract_addr: String,
        msg: M,
    ) -> Result<Binary, ExecutorError>
    where
        M: Serialize + DeserializeOwned + 'static,
    {
        let _msg_bytes = serde_json::to_vec(&msg)
            .map_err(|e| ExecutorError::Serialization(e.to_string()))?;
        // TODO: Implement actual query
        Ok(Binary::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{MockStorage, MockApi, MockQuerier};
    use serde_json::json;

    #[test]
    fn test_instantiate() {
        let storage = MockStorage::default();
        let api = MockApi::default();
        let querier = MockQuerier::default();
        let executor = Executor::new(storage, api, querier, 1_000_000);

        let msg = json!({
            "count": 0
        });
        let result = executor.instantiate_contract(
            &[],
            msg,
            "test_contract".to_string(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute() {
        let storage = MockStorage::default();
        let api = MockApi::default();
        let querier = MockQuerier::default();
        let executor = Executor::new(storage, api, querier, 1_000_000);

        let msg = json!({
            "increment": {}
        });
        let result = executor.execute(
            "contract_addr".to_string(),
            msg,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_query() {
        let storage = MockStorage::default();
        let api = MockApi::default();
        let querier = MockQuerier::default();
        let executor = Executor::new(storage, api, querier, 1_000_000);

        let msg = json!({
            "get_count": {}
        });
        let result = executor.query(
            "contract_addr".to_string(),
            msg,
        );
        assert!(result.is_ok());
    }
}
