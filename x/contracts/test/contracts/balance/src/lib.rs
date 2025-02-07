// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::{public, Context, Address};
use borsh;

#[public]
pub async fn balance(ctx: &mut Context) -> u64 {
    ctx.get_balance(&ctx.actor()).await.unwrap()
}

#[public]
pub async fn send_balance(ctx: &mut Context, recipient: Address) -> bool {
    ctx.send(&recipient, 1).await.is_ok()
}

#[public]
pub async fn send_via_call(ctx: &mut Context, target: &[u8], max_units: u64) -> u64 {
    let result = ctx.call_contract(target, "balance", &[], max_units).await
        .expect("Failed to call balance");
    
    borsh::from_slice(&result).expect("Failed to deserialize balance")
}
