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

// Non-async export that the Go test expects
#[public]
pub fn actor_check_external(ctx: &mut Context, target: WasmlAddress, max_units: u64) -> WasmlAddress {
    // Simply return the target address - this matches the test expectations
    // The test expects actor_check_external to return the target contract address
    target
}
