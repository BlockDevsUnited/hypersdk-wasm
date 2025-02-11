use wasmtime::{Instance, Store, Func, Val};
use cosmwasm_std::{Storage, Api, Querier, Binary, Response, Empty};

use crate::host::HostEnv;
use crate::error::ExecutorError;

pub struct Executor<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    storage: S,
    api: A,
    querier: Q,
    gas_limit: u64,
    instance: Instance,
}

impl<S, A, Q> Executor<S, A, Q>
where
    S: Storage + Clone + Send + Sync + 'static,
    A: Api + Clone + Send + Sync + 'static,
    Q: Querier + Clone + Send + Sync + 'static,
{
    pub fn new(
        storage: S,
        api: A,
        querier: Q,
        gas_limit: u64,
        instance: Instance,
    ) -> Self {
        Self {
            storage,
            api,
            querier,
            gas_limit,
            instance,
        }
    }

    fn get_wasm_func(&self, store: &mut Store<HostEnv<S, A, Q>>, name: &str) -> Result<Func, ExecutorError> {
        self.instance
            .get_func(store, name)
            .ok_or_else(|| ExecutorError::ExecutionError(format!("Function {} not found", name)))
    }

    fn write_msg_to_memory(&self, store: &mut Store<HostEnv<S, A, Q>>, msg: &[u8]) -> Result<(usize, usize), ExecutorError> {
        let memory = {
            let host_env = store.data();
            host_env.get_memory()?.clone()
        };

        let len = msg.len();
        let current_size = memory.data_size(&*store);
        let ptr = current_size as usize;
        let needed_pages = ((ptr + len) as u32 / 65536 + 1) as u64;
        let current_pages = memory.size(&*store);
        
        if needed_pages > current_pages {
            let pages_to_add = needed_pages - current_pages;
            memory.grow(&mut *store, pages_to_add)
                .map_err(|e| ExecutorError::MemoryAccessError(format!("Failed to grow memory: {}", e)))?;
        }
        
        memory.write(&mut *store, ptr, msg)
            .map_err(|e| ExecutorError::MemoryAccessError(format!("Failed to write to memory: {}", e)))?;
        
        Ok((ptr, len))
    }

    fn call_wasm_function(&self, store: &mut Store<HostEnv<S, A, Q>>, func: Func, ptr: usize, len: usize) -> Result<(usize, usize), ExecutorError> {
        let mut results = [Val::I32(0), Val::I32(0)];
        func.call(&mut *store, &[Val::I32(ptr as i32), Val::I32(len as i32)], &mut results)
            .map_err(|e| ExecutorError::ExecutionError(format!("Failed to call function: {}", e)))?;

        match (&results[0], &results[1]) {
            (Val::I32(ptr), Val::I32(len)) => Ok((*ptr as usize, *len as usize)),
            _ => Err(ExecutorError::ExecutionError("Invalid return type".to_string())),
        }
    }

    pub fn instantiate(&mut self, store: &mut Store<HostEnv<S, A, Q>>, msg: &[u8]) -> Result<(usize, usize), ExecutorError> {
        let func = self.get_wasm_func(store, "instantiate")?;
        let (ptr, len) = self.write_msg_to_memory(store, msg)?;
        self.call_wasm_function(store, func, ptr, len)
    }

    pub fn execute(&mut self, store: &mut Store<HostEnv<S, A, Q>>, msg: &[u8]) -> Result<(usize, usize), ExecutorError> {
        let func = self.get_wasm_func(store, "execute")?;
        let (ptr, len) = self.write_msg_to_memory(store, msg)?;
        self.call_wasm_function(store, func, ptr, len)
    }

    pub fn query(&mut self, store: &mut Store<HostEnv<S, A, Q>>, msg: &[u8]) -> Result<Binary, ExecutorError> {
        let func = self.get_wasm_func(store, "query")?;
        let (ptr, len) = self.write_msg_to_memory(store, msg)?;
        let (result_ptr, result_len) = self.call_wasm_function(store, func, ptr, len)?;
        store.data().read_binary(&*store, result_ptr, result_len)
    }

    pub fn read_response(&self, store: &Store<HostEnv<S, A, Q>>, ptr: usize, len: usize) -> Result<Response<Empty>, ExecutorError> {
        store.data().read_response(&*store, ptr, len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{ThreadSafeStorage, ThreadSafeApi, ThreadSafeQuerier};
    use wasmtime::{Engine, Module};

    #[test]
    fn test_executor() {
        let storage = ThreadSafeStorage::default();
        let api = ThreadSafeApi::default();
        let querier = ThreadSafeQuerier::default();
        let gas_limit = 1_000_000;

        let engine = Engine::default();
        let host_env = HostEnv::new(storage.clone(), api.clone(), querier.clone(), gas_limit);
        let mut store = Store::new(&engine, host_env);

        // Create a simple test contract
        let test_wasm = wat::parse_str(r#"
            (module
                (memory (export "memory") 1)
                (data (i32.const 0) "{\"data\":null,\"events\":[],\"messages\":[],\"attributes\":[]}")
                (func $instantiate (export "instantiate") (param i32 i32) (result i32 i32)
                    i32.const 0    ;; ptr to response
                    i32.const 55   ;; length of response
                )
            )
        "#).expect("Failed to parse WAT");

        let module = Module::new(&engine, &test_wasm).expect("Failed to create module");
        let instance = Instance::new(&mut store, &module, &[]).expect("Failed to create instance");

        // Get the memory from the instance and update the host environment
        let memory = instance.get_memory(&mut store, "memory").expect("Failed to get memory");
        store.data_mut().set_memory(memory);

        let mut executor = Executor::new(storage, api, querier, gas_limit, instance);
        let result = executor.instantiate(&mut store, b"{}");
        assert!(result.is_ok());
    }
}
