use cosmwasm_std::{Binary, Storage, Api, Querier};
use serde::Serialize;

use crate::{error::ExecutorError, executor::Executor};

pub mod error;
pub mod executor;
pub mod hash;
pub mod host;

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

    pub fn instantiate(&mut self, code: &[u8], storage: S, api: A, querier: Q) -> Result<(), ExecutorError> {
        let gas_limit = 1_000_000; // Default gas limit
        self.executor = Some(Executor::new(storage, api, querier, gas_limit)?);
        if let Some(executor) = &mut self.executor {
            executor.instantiate(code)?;
        }
        Ok(())
    }

    pub fn instantiate_contract<T: Serialize>(
        &mut self,
        code: &[u8],
        msg: T,
        label: String,
    ) -> Result<Binary, ExecutorError> {
        match &mut self.executor {
            Some(executor) => {
                let result = executor.instantiate_contract(code, msg, label)?;
                Ok(Binary::from(result))
            }
            None => Err(ExecutorError::Instantiation(
                "Contract not instantiated".to_string(),
            )),
        }
    }

    pub fn execute_contract<T: Serialize>(
        &mut self,
        contract_addr: &str,
        msg: T,
    ) -> Result<Binary, ExecutorError> {
        match &mut self.executor {
            Some(executor) => {
                let result = executor.execute_contract(contract_addr, msg)?;
                Ok(Binary::from(result))
            }
            None => Err(ExecutorError::Instantiation(
                "Contract not instantiated".to_string(),
            )),
        }
    }

    pub fn query_contract<T: Serialize>(
        &mut self,
        contract_addr: &str,
        msg: T,
    ) -> Result<Binary, ExecutorError> {
        match &mut self.executor {
            Some(executor) => {
                let result = executor.query_contract(contract_addr, msg)?;
                Ok(Binary::from(result))
            }
            None => Err(ExecutorError::Instantiation(
                "Contract not instantiated".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    // Removed unused import
}
