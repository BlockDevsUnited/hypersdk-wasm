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

#[public]
pub async fn actor_check_external(ctx: &mut Context, _target: Vec<u8>, _max_units: u64) -> WasmlAddress {
    // Return the actual actor address from the context
    ctx.actor.clone()
}
