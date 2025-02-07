// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{public, Context, Address, Gas};

#[public]
pub fn simple_call(_: &mut Context) -> i64 {
    0
}

#[public]
pub async fn simple_call_external(ctx: &mut Context, target: &[u8], max_units: u64) -> i64 {
    let result = ctx.call_contract(target, "simple_call", &[], max_units).await
        .expect("Failed to call simple_call");
    
    borsh::from_slice(&result).expect("Failed to deserialize result")
}

#[public]
pub fn actor_check(context: &mut Context) -> Address {
    context.actor().into()
}

#[public]
pub async fn actor_check_external(ctx: &mut Context, target: &[u8], max_units: u64) -> Address {
    let result = ctx.call_contract(target, "actor_check", &[], max_units).await
        .expect("Failed to call actor_check");
    
    borsh::from_slice(&result).expect("Failed to deserialize address")
}

#[public]
pub fn call_with_param(_: &mut Context, value: i64) -> i64 {
    value
}

#[public]
pub async fn call_with_param_external(
    ctx: &mut Context,
    target: &[u8],
    value: i64,
    max_units: u64,
) -> i64 {
    let args = borsh::to_vec(&value).expect("Failed to serialize value");
    let result = ctx.call_contract(target, "call_with_param", &args, max_units).await
        .expect("Failed to call call_with_param");
    
    borsh::from_slice(&result).expect("Failed to deserialize result")
}

#[public]
pub fn call_with_two_params(_: &mut Context, value1: i64, value2: i64) -> i64 {
    value1 + value2
}

#[public]
pub async fn call_with_two_params_external(
    ctx: &mut Context,
    target: &[u8],
    value1: i64,
    value2: i64,
    max_units: u64,
) -> i64 {
    #[derive(BorshSerialize, BorshDeserialize)]
    struct Args {
        value1: i64,
        value2: i64,
    }

    let args = borsh::to_vec(&Args { value1, value2 }).expect("Failed to serialize args");
    let result = ctx.call_contract(target, "call_with_two_params", &args, max_units).await
        .expect("Failed to call call_with_two_params");
    
    borsh::from_slice(&result).expect("Failed to deserialize result")
}
