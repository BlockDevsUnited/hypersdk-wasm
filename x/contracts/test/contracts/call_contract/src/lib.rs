// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::{
    Context,
    types::{Address, WasmlAddress},
};
use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct State {
    value: u64,
}

#[no_mangle]
pub fn actor_check(context: &mut Context) -> Address {
    let addr = context.actor();
    let bytes: [u8; 33] = addr.as_bytes().try_into().expect("Invalid address length");
    Address::from(bytes)
}

#[no_mangle]
pub async fn call_contract(context: &mut Context, contract: Address, value: u64) {
    let state = State { value };
    let serialized = borsh::to_vec(&state).expect("Failed to serialize state");
    context.call_contract(&contract.as_bytes(), "store", &serialized, 0)
        .await
        .expect("Failed to call contract");
}

#[no_mangle]
pub async fn get_result_value(context: &mut Context) -> u64 {
    let result = context.call_contract(&[], "get_value", &[], 0)
        .await
        .expect("Failed to get result");
    let state = State::try_from_slice(&result)
        .expect("Failed to deserialize state");
    state.value
}

#[no_mangle]
pub fn simple_call(_: &mut Context) -> i64 {
    0
}

#[no_mangle]
pub async fn simple_call_external(ctx: &mut Context, target: &[u8], max_units: u64) -> i64 {
    let result = ctx.call_contract(target, "simple_call", &[], max_units)
        .await
        .expect("Failed to call simple_call");
    if result.len() >= 8 {
        i64::from_le_bytes(result[0..8].try_into().unwrap())
    } else {
        0
    }
}

#[no_mangle]
pub async fn actor_check_external(ctx: &mut Context, target: &[u8], max_units: u64) -> Address {
    let result = ctx.call_contract(target, "actor_check", &[], max_units)
        .await
        .expect("Failed to call actor_check");
    let bytes: [u8; 33] = result[0..33].try_into().expect("Invalid address length");
    Address::from(bytes)
}

#[no_mangle]
pub fn call_with_param(_: &mut Context, value: i64) -> i64 {
    value
}

#[no_mangle]
pub async fn call_with_param_external(
    ctx: &mut Context,
    target: &[u8],
    value: i64,
    max_units: u64,
) -> i64 {
    let args = value.to_le_bytes();
    let result = ctx.call_contract(target, "call_with_param", &args, max_units)
        .await
        .expect("Failed to call call_with_param");
    i64::from_le_bytes(result[0..8].try_into().expect("Invalid result length"))
}

#[no_mangle]
pub fn call_with_two_params(_: &mut Context, value1: i64, value2: i64) -> i64 {
    value1 + value2
}

#[no_mangle]
pub async fn call_with_two_params_external(
    ctx: &mut Context,
    target: &[u8],
    value1: i64,
    value2: i64,
    max_units: u64,
) -> i64 {
    let mut args = Vec::with_capacity(16);
    args.extend_from_slice(&value1.to_le_bytes());
    args.extend_from_slice(&value2.to_le_bytes());
    let result = ctx.call_contract(target, "call_with_two_params", &args, max_units)
        .await
        .expect("Failed to call call_with_two_params");
    i64::from_le_bytes(result[0..8].try_into().expect("Invalid result length"))
}

#[no_mangle]
pub async fn call(context: &mut Context, target: Address, function: &str, args: &[u8], max_units: u64) -> Vec<u8> {
    context.call_contract(&target.as_bytes(), function, args, max_units)
        .await
        .expect("Failed to call contract")
}

#[no_mangle]
pub async fn call_with_value(context: &mut Context, target: Address, function: &str, args: &[u8], max_units: u64, value: u64) -> Vec<u8> {
    let from = context.actor().clone();
    let to = WasmlAddress::try_from(target.as_bytes()).expect("Invalid target address");
    context.transfer(&from, &to, value).await.unwrap();
    
    context.call_contract(&target.as_bytes(), function, args, max_units)
        .await
        .expect("Failed to call contract")
}
