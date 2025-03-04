// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::vec::Vec;
use sdk_macros::public;
use wasmlanche::Context;

#[public]
pub fn simple_call(_: &mut Context) -> i64 {
    0
}

#[public]
pub async fn simple_call_external(_ctx: &mut Context, _target: Vec<u8>, _max_units: u64) -> i64 {
    // Simulated call and result since call_contract is not available
    // This is a placeholder until the actual interface is updated
    0 // Just return 0 directly
}
