#![cfg_attr(not(feature = "library"), no_main)]

use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    to_json_binary, from_json, entry_point,
};
use serde::{Deserialize, Serialize};

// State

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub count: i32,
}

// Messages

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

// Contract implementation

#[derive(Serialize, Deserialize, Debug)]
pub struct CountResponse {
    pub count: i32,
}

const STATE_KEY: &[u8] = b"state";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State { count: msg.count };
    deps.storage.set(STATE_KEY, &to_json_binary(&state)?);
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Increment {} => {
            let mut state: State = from_json(&deps.storage.get(STATE_KEY).unwrap())?;
            state.count += 1;
            deps.storage.set(STATE_KEY, &to_json_binary(&state)?);
            Ok(Response::default())
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => {
            let state: State = from_json(&deps.storage.get(STATE_KEY).unwrap())?;
            to_json_binary(&CountResponse { count: state.count })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { count: 17 };

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_json(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { count: 17 };

        // Initialize the contract
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Execute increment
        let res = execute(deps.as_mut(), env.clone(), info, ExecuteMsg::Increment {}).unwrap();
        assert_eq!(0, res.messages.len());

        // Query the new count
        let res = query(deps.as_ref(), env, QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_json(&res).unwrap();
        assert_eq!(18, value.count);
    }
}
