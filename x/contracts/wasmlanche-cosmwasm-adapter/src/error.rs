use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Contract instantiation failed: {0}")]
    Instantiation(String),

    #[error("Contract execution failed: {0}")]
    Execution(String),

    #[error("Gas limit exceeded: {0}")]
    GasLimit(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Entry point not found: {0}")]
    EntryPoint(String),

    #[error("Memory access error: {0}")]
    Memory(String),

    #[error("Host function error: {0}")]
    HostFunction(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Contract not found: {0}")]
    ContractNotFound(String),

    #[error("Instance not found")]
    InstanceNotFound,
}

#[derive(Error, Debug)]
pub enum HostError {
    #[error("Memory allocation failed: {0}")]
    Allocation(String),

    #[error("Memory access failed: {0}")]
    MemoryAccess(String),

    #[error("Gas limit exceeded: {0}")]
    GasLimit(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Query error: {0}")]
    Query(String),
}

impl From<HostError> for ExecutorError {
    fn from(err: HostError) -> Self {
        match err {
            HostError::Allocation(msg) => ExecutorError::Memory(msg),
            HostError::MemoryAccess(msg) => ExecutorError::Memory(msg),
            HostError::GasLimit(msg) => ExecutorError::GasLimit(msg),
            HostError::Storage(msg) => ExecutorError::Storage(msg),
            HostError::Api(msg) => ExecutorError::HostFunction(msg),
            HostError::Query(msg) => ExecutorError::Query(msg),
        }
    }
}

impl From<wasmtime::Error> for ExecutorError {
    fn from(err: wasmtime::Error) -> Self {
        ExecutorError::Execution(err.to_string())
    }
}

impl From<serde_json::Error> for ExecutorError {
    fn from(err: serde_json::Error) -> Self {
        ExecutorError::Serialization(err.to_string())
    }
}
