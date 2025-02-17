use thiserror::Error;
use std::error::Error as StdError;

#[derive(Error, Debug)]
pub enum ExecutorError {
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

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Memory allocation error: {0}")]
    MemoryError(String),

    #[error("No contract loaded")]
    NoContract,

    #[error("Gas limit exceeded")]
    GasLimitExceeded,
}

impl From<wasmtime::Error> for ExecutorError {
    fn from(err: wasmtime::Error) -> Self {
        ExecutorError::ExecutionError(err.to_string())
    }
}

impl From<serde_json::Error> for ExecutorError {
    fn from(err: serde_json::Error) -> Self {
        ExecutorError::SerializationError(err.to_string())
    }
}

impl From<cosmwasm_std::StdError> for ExecutorError {
    fn from(err: cosmwasm_std::StdError) -> Self {
        ExecutorError::ApiError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::StdError;

    #[test]
    fn test_error_conversion() {
        // Create a wasmtime error using anyhow
        let wasm_err = wasmtime::Error::msg("test error");
        let exec_err: ExecutorError = wasm_err.into();
        assert!(matches!(exec_err, ExecutorError::ExecutionError(_)));

        let std_err = StdError::generic_err("test error");
        let contract_err: ExecutorError = std_err.into();
        assert!(matches!(contract_err, ExecutorError::ApiError(_)));

        let json_err = serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        let ser_err: ExecutorError = json_err.into();
        assert!(matches!(ser_err, ExecutorError::SerializationError(_)));
    }
}
