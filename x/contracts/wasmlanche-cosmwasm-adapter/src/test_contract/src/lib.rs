use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, StdError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize)]
pub struct ExecuteMsg {
    pub value: i32,
}

#[derive(Serialize, Deserialize)]
pub struct QueryMsg {
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub struct ExecuteResponse {
    pub result: i32,
}

#[derive(Serialize, Deserialize)]
pub struct QueryResponse {
    pub value: String,
}

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    Ok(Response::new().add_attribute("result", msg.value.to_string()))
}

#[entry_point]
pub fn query(
    _deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    let response = QueryResponse { value: msg.key };
    serde_json::to_vec(&response)
        .map(Binary::from)
        .map_err(|e| StdError::generic_err(format!("Serialization error: {}", e)))
}
