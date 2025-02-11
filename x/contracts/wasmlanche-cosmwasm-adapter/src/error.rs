use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostError {
    #[error("Gas limit error: {0}")]
    GasLimit(String),
    
    #[error("Memory access error: {0}")]
    MemoryAccess(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Querier error: {0}")]
    Querier(String),
}

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Module creation error: {0}")]
    ModuleCreation(String),
    
    #[error("Instantiation error: {0}")]
    Instantiation(String),
    
    #[error("Execution error: {0}")]
    Execution(String),
    
    #[error("Host error: {0}")]
    Host(#[from] HostError),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Wasmtime error: {0}")]
    WasmtimeError(String),
}
