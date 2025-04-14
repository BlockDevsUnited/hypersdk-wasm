// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::vec::Vec;
// Don't use the macro for now
// use sdk_macros::public;
use wasmlanche::Context;

// Standard sync function - no macro
pub fn call_with_param(_: &mut Context, value: i64) -> i64 {
    // Return the value as is
    value
}

// The async function can still use the macro if needed
// for now we'll use a standard definition
pub async fn call_with_param_external(
    _ctx: &mut Context,
    _target: Vec<u8>,
    value: i64,
    _max_units: u64,
) -> i64 {
    // Simulated call and result since call_contract is not available
    value // Just return the value directly
}
