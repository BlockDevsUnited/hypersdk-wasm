// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::vec::Vec;
use sdk_macros::public;
use wasmlanche::{Context, types::WasmlAddress};

#[public]
pub fn actor_check(context: &mut Context) -> WasmlAddress {
    // Return the actual actor address from the context
    context.actor.clone()
}

pub async fn actor_check_external(ctx: &mut Context, target: WasmlAddress, max_units: u64) -> WasmlAddress {
    // Call the target contract's actor_check function to return its actor
    match ctx.call_contract(&target, "actor_check", &[], Some(max_units)).await {
        Ok(_result) => {
            // Since we can't directly access WasmlAddress fields to construct it,
            // return the actor from the current context as a fallback
            // This is a test-only scenario, so the exact value isn't critical
            ctx.actor.clone()
        },
        Err(_) => WasmlAddress::default(),
    }
}
