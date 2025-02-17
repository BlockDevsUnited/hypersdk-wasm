use cosmwasm_std::{
    to_json_vec, Addr, Binary, ContractResult, QueryRequest, StdResult, SystemResult,
    RecoverPubkeyError, VerificationError, WasmQuery, Empty, Storage, Api, Querier,
    CanonicalAddr, Order, Record, QuerierResult, MessageInfo, to_binary, to_json_binary, from_json,
};
use wasmlanche_cosmwasm_adapter::WasmAdapter;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use anyhow::Result;
use serde_json;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct ClonableStorage {
    inner: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>
}

impl ClonableStorage {
    fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new()))
        }
    }
}

impl Storage for ClonableStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.inner.read().unwrap().get(key).cloned()
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.inner.write().unwrap().insert(key.to_vec(), value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        self.inner.write().unwrap().remove(key);
    }

    fn range<'a>(&'a self, start: Option<&[u8]>, end: Option<&[u8]>, order: Order) -> Box<dyn Iterator<Item = Record> + 'a> {
        let guard = self.inner.read().unwrap();
        let records: Vec<Record> = guard.iter()
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
            .collect();

        match order {
            Order::Ascending => Box::new(records.into_iter()),
            Order::Descending => Box::new(records.into_iter().rev())
        }
    }
}

#[derive(Clone)]
struct ClonableApi;

impl ClonableApi {
    fn new() -> Self {
        Self
    }
}

impl Api for ClonableApi {
    fn addr_validate(&self, human: &str) -> StdResult<Addr> {
        Ok(Addr::unchecked(human))
    }

    fn addr_canonicalize(&self, human: &str) -> StdResult<CanonicalAddr> {
        Ok(CanonicalAddr::from(human.as_bytes()))
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        Ok(Addr::unchecked(String::from_utf8_lossy(canonical.as_slice())))
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

    fn debug(&self, _message: &str) {
        // Do nothing
    }
}

#[derive(Clone)]
struct ClonableQuerier;

impl ClonableQuerier {
    fn new() -> Self {
        Self
    }
}

impl Querier for ClonableQuerier {
    fn raw_query(&self, _bin_request: &[u8]) -> QuerierResult {
        SystemResult::Ok(ContractResult::Ok(Binary::default()))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CounterQueryMsg {
    GetCount {},
}

#[derive(Serialize, Deserialize)]
struct CountResponse {
    count: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = WasmAdapter::new(
        ClonableStorage::new(),
        ClonableApi::new(),
        ClonableQuerier::new(),
        1_000_000,
    );

    // Load and instantiate the contract
    let wasm = std::fs::read("counter.wasm")?;
    let code_id = adapter.store_code(&wasm)?;
    println!("Stored contract with code ID: {:?}", code_id);

    let init_msg = serde_json::json!({
        "initial_count": 0
    });

    let info = MessageInfo {
        sender: Addr::unchecked("sender"),
        funds: vec![],
    };

    let init_msg_bytes = to_json_binary(&init_msg)?;
    let contract_addr = adapter.instantiate(&init_msg_bytes, info.clone(), None)?;
    let contract_addr_str = String::from_utf8(contract_addr.clone())?;
    println!("Instantiated contract at address: {}", contract_addr_str);

    // Execute increment
    let exec_msg = serde_json::json!({
        "increment": {}
    });
    let exec_msg_bytes = to_json_binary(&exec_msg)?;
    adapter.execute(&exec_msg_bytes, info, None)?;

    // Query count
    let query: QueryRequest<Empty> = QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr_str,
        msg: to_json_binary(&CounterQueryMsg::GetCount {})?,
    });

    let response = adapter.query(&query)?;
    match response {
        ContractResult::Ok(data) => {
            let count_response: CountResponse = from_json(&data)?;
            println!("Count: {}", count_response.count);
        }
        ContractResult::Err(err) => {
            println!("Error: {}", err);
        }
    }

    Ok(())
}
