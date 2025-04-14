// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::Context;

const STATE_KEY: &[u8] = b"state_value";

#[public]
pub fn modify_state(ctx: &mut Context, value: &str) -> i64 {
    // Store the value in state
    ctx.get_state().unwrap().set(STATE_KEY, value.as_bytes());
    0
}

#[public]
pub fn get_state(ctx: &mut Context) -> i64 {
    // Return 1 if state exists, 0 otherwise
    if ctx.get_state().unwrap().get(STATE_KEY).is_some() {
        1
    } else {
        0
    }
}
