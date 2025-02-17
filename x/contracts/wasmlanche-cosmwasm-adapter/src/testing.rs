use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use cosmwasm_std::{
    Binary, Storage, Order, Api, Querier, ContractResult, SystemResult, Empty, WasmQuery,
    testing::{MockStorage, MockQuerier},
};

// Thread-safe function type aliases
type ThreadSafeEmptyQueryFn = Arc<dyn Fn(&Empty) -> SystemResult<ContractResult<Binary>> + Send + Sync>;
type ThreadSafeWasmQueryFn = Arc<dyn Fn(&WasmQuery) -> SystemResult<ContractResult<Binary>> + Send + Sync>;

#[derive(Clone)]
pub struct ThreadSafeStorage(Arc<RwLock<MockStorage>>);

impl Default for ThreadSafeStorage {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(MockStorage::default())))
    }
}

impl Storage for ThreadSafeStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.read().unwrap().get(key)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.0.write().unwrap().set(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.0.write().unwrap().remove(key)
    }

    fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let data = self.0.read().unwrap();
        let items: Vec<_> = data.range(start, end, order).collect();
        Box::new(items.into_iter())
    }
}

#[derive(Clone)]
pub struct ThreadSafeApi;

impl Default for ThreadSafeApi {
    fn default() -> Self {
        Self
    }
}

impl Api for ThreadSafeApi {
    fn addr_validate(&self, human: &str) -> cosmwasm_std::StdResult<cosmwasm_std::Addr> {
        // Basic validation: must start with "cosmos" and be at least 10 chars
        if !human.starts_with("cosmos") || human.len() < 10 {
            return Err(cosmwasm_std::StdError::generic_err("Invalid address"));
        }
        Ok(cosmwasm_std::Addr::unchecked(human))
    }

    fn addr_canonicalize(&self, human: &str) -> cosmwasm_std::StdResult<cosmwasm_std::CanonicalAddr> {
        // Validate the address first
        self.addr_validate(human)?;
        Ok(cosmwasm_std::CanonicalAddr::from(human.as_bytes().to_vec()))
    }

    fn addr_humanize(&self, canonical: &cosmwasm_std::CanonicalAddr) -> cosmwasm_std::StdResult<cosmwasm_std::Addr> {
        // Convert the canonical address back to a string
        let human = String::from_utf8(canonical.as_slice().to_vec())
            .map_err(|_| cosmwasm_std::StdError::generic_err("Invalid canonical address"))?;
        self.addr_validate(&human)
    }

    fn secp256k1_verify(&self, message_hash: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, cosmwasm_std::VerificationError> {
        // Removed implementation
        unimplemented!()
    }

    fn secp256k1_recover_pubkey(&self, message_hash: &[u8], signature: &[u8], recovery_param: u8) -> Result<Vec<u8>, cosmwasm_std::RecoverPubkeyError> {
        // Removed implementation
        unimplemented!()
    }

    fn ed25519_verify(&self, message: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, cosmwasm_std::VerificationError> {
        // Removed implementation
        unimplemented!()
    }

    fn ed25519_batch_verify(&self, messages: &[&[u8]], signatures: &[&[u8]], public_keys: &[&[u8]]) -> Result<bool, cosmwasm_std::VerificationError> {
        // Removed implementation
        unimplemented!()
    }

    fn debug(&self, message: &str) {
        // Removed implementation
        unimplemented!()
    }
}

#[derive(Clone)]
pub struct ThreadSafeQuerier {
    empty_handler: ThreadSafeEmptyQueryFn,
    wasm_handler: ThreadSafeWasmQueryFn,
}

impl Default for ThreadSafeQuerier {
    fn default() -> Self {
        Self {
            empty_handler: Arc::new(|_| SystemResult::Ok(ContractResult::Ok(Binary::default()))),
            wasm_handler: Arc::new(|_| SystemResult::Ok(ContractResult::Ok(Binary::default()))),
        }
    }
}

impl ThreadSafeQuerier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_empty<F>(mut self, handler: F) -> Self 
    where
        F: Fn(&Empty) -> SystemResult<ContractResult<Binary>> + Send + Sync + 'static,
    {
        self.empty_handler = Arc::new(handler);
        self
    }

    pub fn with_wasm<F>(mut self, handler: F) -> Self 
    where
        F: Fn(&WasmQuery) -> SystemResult<ContractResult<Binary>> + Send + Sync + 'static,
    {
        self.wasm_handler = Arc::new(handler);
        self
    }
}

impl Querier for ThreadSafeQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> cosmwasm_std::QuerierResult {
        // Default implementation - you may want to customize this based on your needs
        SystemResult::Ok(ContractResult::Ok(Binary::default()))
    }
}
