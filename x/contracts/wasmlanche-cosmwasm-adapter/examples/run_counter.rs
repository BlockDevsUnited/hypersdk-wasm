use cosmwasm_std::{
    Addr, Binary, ContractResult, MessageInfo, QueryRequest, WasmQuery, from_json, to_json_binary,
    StdResult, SystemResult, Storage, Api, Querier, Order, QuerierResult, CanonicalAddr,
    StdError, VerificationError, RecoverPubkeyError, Empty,
};
use wasmlanche_cosmwasm_adapter::WasmAdapter;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use anyhow::Result;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct InstantiateMsg {
    pub count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ExecuteMsg {
    Increment {},
}

#[derive(Serialize, Deserialize, Debug)]
pub enum QueryMsg {
    GetCount {},
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CountResponse {
    pub count: i32,
}

#[derive(Default, Clone)]
struct TestStorage {
    data: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl Storage for TestStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.read().unwrap().get(key).cloned()
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.data.write().unwrap().insert(key.to_vec(), value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        self.data.write().unwrap().remove(key);
    }

    fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let data = self.data.read().unwrap();
        let iter = data
            .iter()
            .filter(move |(k, _)| {
                if let Some(start) = start {
                    if k.as_slice() < start {
                        return false;
                    }
                }
                if let Some(end) = end {
                    if k.as_slice() >= end {
                        return false;
                    }
                }
                true
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();

        match order {
            Order::Ascending => Box::new(iter.into_iter()),
            Order::Descending => Box::new(iter.into_iter().rev()),
        }
    }
}

#[derive(Default, Clone)]
struct TestApi;

impl Api for TestApi {
    fn debug(&self, message: &str) {
        println!("Debug: {}", message);
    }

    fn addr_validate(&self, human: &str) -> StdResult<Addr> {
        Ok(Addr::unchecked(human))
    }

    fn addr_canonicalize(&self, human: &str) -> StdResult<CanonicalAddr> {
        Ok(CanonicalAddr::from(Binary::from(human.as_bytes())))
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        String::from_utf8(canonical.as_slice().to_vec())
            .map(Addr::unchecked)
            .map_err(|_| StdError::generic_err("Invalid canonical address"))
    }

    fn secp256k1_verify(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        Ok(true)
    }

    fn secp256k1_recover_pubkey(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        Ok(vec![])
    }

    fn ed25519_verify(
        &self,
        _message: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        Ok(true)
    }

    fn ed25519_batch_verify(
        &self,
        _messages: &[&[u8]],
        _signatures: &[&[u8]],
        _public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        Ok(true)
    }
}

#[derive(Default, Clone)]
struct TestQuerier;

impl Querier for TestQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<Empty> = match from_json(bin_request) {
            Ok(parsed) => parsed,
            Err(e) => return SystemResult::Ok(ContractResult::Err(e.to_string())),
        };

        match request {
            QueryRequest::Wasm(WasmQuery::Smart { .. }) => {
                SystemResult::Ok(ContractResult::Ok(Binary::default()))
            }
            _ => SystemResult::Ok(ContractResult::Err("Unsupported query type".to_string())),
        }
    }
}

fn main() -> Result<()> {
    // Create test environment
    let storage = TestStorage::default();
    let api = TestApi::default();
    let querier = TestQuerier::default();
    let mut adapter = WasmAdapter::new(storage, api, querier, 100_000_000);

    // Load the Wasm bytecode
    let wasm = fs::read("examples/counter/target/wasm32-unknown-unknown/release/counter.wasm")?;

    // Instantiate the contract with initial count of 5
    let instantiate_msg = InstantiateMsg { count: 5 };
    let info = MessageInfo {
        sender: Addr::unchecked("sender"),
        funds: vec![],
    };
    let instantiate_msg_bytes = to_json_binary(&instantiate_msg)?;

    // Instantiate
    adapter.instantiate(&wasm, &instantiate_msg_bytes, &info, vec![])?;
    println!("Contract instantiated with count: 5");

    // Execute increment
    let execute_msg = ExecuteMsg::Increment {};
    let execute_msg_bytes = to_json_binary(&execute_msg)?;
    adapter.execute(&execute_msg_bytes, &info)?;
    println!("Executed increment");

    // Query count
    let query_msg = QueryMsg::GetCount {};
    let query_msg_bytes = to_json_binary(&query_msg)?;
    let query_request = QueryRequest::<Empty>::Wasm(WasmQuery::Smart {
        contract_addr: "contract".to_string(),
        msg: query_msg_bytes,
    });
    
    match adapter.query(&query_request)? {
        ContractResult::Ok(data) => {
            let response: CountResponse = from_json(&data)?;
            println!("Current count: {}", response.count);
        }
        ContractResult::Err(err) => {
            println!("Query error: {}", err);
        }
    }

    Ok(())
}
