// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::{Context, types::Address};
use wasmlanche::borsh;

#[public]
pub async fn balance(ctx: &mut Context) -> u64 {
    ctx.get_balance(&ctx.actor()).await.unwrap()
}

#[public]
pub async fn send(ctx: &mut Context, recipient: &[u8], amount: u64) -> bool {
    ctx.send(&recipient, amount).await.is_ok()
}

#[public]
pub async fn send_via_call(ctx: &mut Context, target: &[u8], max_units: u64) -> u64 {
    let result = ctx.call_contract(target, "balance", &[], max_units).await
        .expect("Failed to call balance");
    
    borsh::BorshDeserialize::try_from_slice(&result).expect("Failed to deserialize balance")
}
