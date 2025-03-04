// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![no_std]
extern crate alloc;

#[cfg(not(target_arch = "wasm32"))]
extern crate std;

use sdk_macros::public;
use wasmlanche::Context;

#[public]
pub fn fail(_ctx: &mut Context) -> i64 {
    panic!("Simulated contract failure");
}

#[public]
pub fn get_value(ctx: &mut Context) -> i64 {
    // Access the value from contract call context
    if ctx.value() > 0 {
        // Return the actual value that was attached
        ctx.value() as i64
    } else {
        0
    }
}

// Async variant of get_value
#[public]
pub async fn get_value_async(_ctx: &mut Context) -> i64 {
    42
}
