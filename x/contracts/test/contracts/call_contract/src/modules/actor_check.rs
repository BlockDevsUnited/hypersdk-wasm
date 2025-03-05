// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::vec::Vec;
// Don't use the macro for now
// use sdk_macros::public;
use wasmlanche::{Context, types::WasmlAddress};

// Standard sync function - no macro
pub fn actor_check(context: &mut Context) -> WasmlAddress {
    // Return the actual actor address from the context
    context.actor.clone()
}

// The async function can still use the macro if needed
// for now we'll use a standard definition
pub async fn actor_check_external(ctx: &mut Context, _target: Vec<u8>, _max_units: u64) -> WasmlAddress {
    // Return the actual actor address from the context
    ctx.actor.clone()
}
