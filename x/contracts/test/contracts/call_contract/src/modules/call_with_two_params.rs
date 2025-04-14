// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::vec::Vec;
// Don't use the macro for now
// use sdk_macros::public;
use wasmlanche::Context;

// This function adds two values
// Standard sync function - no macro
pub fn call_with_two_params(_: &mut Context, value1: i64, value2: i64) -> i64 {
    // Return the sum of two values
    value1 + value2
}

// The async function can still use the macro if needed
// for now we'll use a standard definition
pub async fn call_with_two_params_external(
    _ctx: &mut Context,
    _target: Vec<u8>,
    value1: i64,
    value2: i64,
    _max_units: u64,
) -> i64 {
    // Simulated call and result since call_contract is not available
    value1 + value2 // Just add and return directly
}
