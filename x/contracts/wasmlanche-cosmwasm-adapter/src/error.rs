use thiserror::Error;
use cosmwasm_std::{StdError as CosmWasmStdError, RecoverPubkeyError};
use anyhow;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Cryptographic error: {0}")]
    CryptoError(#[from] RecoverPubkeyError),

    #[error("CosmWasm error: {0}")]
    CosmWasmError(#[from] CosmWasmStdError),

    #[error("Host function error: {0}")]
    HostFunctionError(String),

    #[error("Memory access error: {0}")]
    MemoryAccessError(String),

    #[error("No memory available")]
    NoMemory,

    #[error("No WASM module loaded")]
    NoModule,

    #[error("Module error: {0}")]
    ModuleError(String),

    #[error("Instantiation error: {0}")]
    InstantiationError(String),

    #[error("Entry point not found: {0}")]
    EntryPointNotFound(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Memory allocation error: {0}")]
    MemoryError(String),

    #[error("No contract loaded")]
    NoContract,

    #[error("Gas limit exceeded")]
    GasLimitExceeded,

    #[error("Contract not instantiated")]
    NotInstantiated,

    #[error("No memory export found")]
    NoMemoryExport,
}

impl From<anyhow::Error> for ExecutorError {
    fn from(err: anyhow::Error) -> Self {
        ExecutorError::RuntimeError(err.to_string())
    }
}

impl From<serde_json::Error> for ExecutorError {
    fn from(err: serde_json::Error) -> Self {
        ExecutorError::SerializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::StdError as CosmWasmStdError;

    #[test]
    fn test_error_conversion() {
        let std_err = CosmWasmStdError::generic_err("test error");
        let contract_err: ExecutorError = std_err.into();
        assert!(matches!(contract_err, ExecutorError::CosmWasmError(_)));

        let json_err = serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        let ser_err: ExecutorError = json_err.into();
        assert!(matches!(ser_err, ExecutorError::SerializationError(_)));
    }
}
