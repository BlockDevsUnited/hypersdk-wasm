// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::{public, Context};

#[public]
pub async fn out_of_fuel(ctx: &mut Context, target: &[u8]) -> bool {
    // Call with insufficient gas to trigger out of fuel error
    ctx.call_contract(target, "simple_call", &[], 1).await.is_err()
}
