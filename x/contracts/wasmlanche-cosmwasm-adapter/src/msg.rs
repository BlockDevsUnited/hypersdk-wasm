use cosmwasm_std::{Binary, ContractResult, Response, SystemResult};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub code_id: u64,
    pub msg: Binary,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ExecuteMsg {
    pub contract_addr: String,
    pub msg: Binary,
    pub funds: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct QueryMsg {
    pub contract_addr: String,
    pub msg: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MigrateMsg {
    pub contract_addr: String,
    pub new_code_id: u64,
    pub msg: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Coin {
    pub denom: String,
    pub amount: u128,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractResponse {
    pub data: Option<Binary>,
    pub events: Vec<Event>,
    pub messages: Vec<String>,
    pub attributes: Vec<EventAttribute>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub r#type: String,
    pub attributes: Vec<EventAttribute>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EventAttribute {
    pub key: String,
    pub value: String,
}

impl From<ContractResponse> for Response {
    fn from(resp: ContractResponse) -> Self {
        let mut response = Response::new();
        if let Some(data) = resp.data {
            response = response.set_data(data);
        }
        for event in resp.events {
            response = response.add_event(event.into());
        }
        for attr in resp.attributes {
            response = response.add_attribute(attr.key, attr.value);
        }
        response
    }
}

impl From<Event> for cosmwasm_std::Event {
    fn from(event: Event) -> Self {
        let mut cosmos_event = cosmwasm_std::Event::new(event.r#type);
        for attr in event.attributes {
            cosmos_event = cosmos_event.add_attribute(attr.key, attr.value);
        }
        cosmos_event
    }
}

// Error handling for message processing
#[derive(Debug)]
pub enum MessageError {
    SerializationError(String),
    ValidationError(String),
    ExecutionError(String),
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessageError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            MessageError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            MessageError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

impl std::error::Error for MessageError {}

// Helper functions for message processing
pub fn parse_response<T>(response: Result<T, String>) -> ContractResult<T> {
    match response {
        Ok(res) => ContractResult::Ok(res),
        Err(err) => ContractResult::Err(err),
    }
}

pub fn parse_system_result(result: SystemResult<ContractResult<Binary>>) -> Result<Binary, MessageError> {
    match result {
        SystemResult::Ok(contract_result) => match contract_result {
            ContractResult::Ok(response) => Ok(response),
            ContractResult::Err(err) => Err(MessageError::ExecutionError(err)),
        },
        SystemResult::Err(err) => Err(MessageError::ExecutionError(err.to_string())),
    }
}
