use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, from_json,
};
use serde::{Deserialize, Serialize};

// State
const COUNT_KEY: &[u8] = b"count";

// Messages
#[derive(Serialize, Deserialize)]
pub struct InstantiateMsg {
    pub initial_count: i32,
}

#[derive(Serialize, Deserialize)]
pub enum ExecuteMsg {
    Increment {},
    Reset { count: i32 },
}

#[derive(Serialize, Deserialize)]
pub enum QueryMsg {
    GetCount {},
}

// Query responses
#[derive(Serialize, Deserialize)]
pub struct CountResponse {
    pub count: i32,
}

// Contract implementation
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    deps.storage.set(COUNT_KEY, &msg.initial_count.to_be_bytes());
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
            let count = get_count(deps.storage)?;
            let new_count = count + 1;
            deps.storage.set(COUNT_KEY, &new_count.to_be_bytes());
            Ok(Response::new().add_attribute("action", "increment"))
        }
        ExecuteMsg::Reset { count } => {
            deps.storage.set(COUNT_KEY, &count.to_be_bytes());
            Ok(Response::new().add_attribute("action", "reset"))
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => {
            let count = get_count(deps.storage)?;
            to_json_binary(&CountResponse { count })
        }
    }
}

fn get_count(storage: &dyn cosmwasm_std::Storage) -> StdResult<i32> {
    let count_bytes = storage.get(COUNT_KEY).unwrap_or_default();
    let count = if count_bytes.is_empty() {
        0i32
    } else {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&count_bytes);
        i32::from_be_bytes(bytes)
    };
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { initial_count: 17 };
        let info = mock_info("creator", &[]);
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_json(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { initial_count: 17 };
        let info = mock_info("creator", &[]);
        let env = mock_env();

        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // increment by 1
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::Increment {};
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_json(&res).unwrap();
        assert_eq!(18, value.count);
    }
}
