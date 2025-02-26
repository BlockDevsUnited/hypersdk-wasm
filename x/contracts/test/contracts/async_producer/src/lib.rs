// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::Context;

const SHARED_KEY: &[u8] = b"shared_value";

#[public]
pub fn produce(ctx: &mut Context) -> i64 {
    // Produce a value and store it in state
    let value: i32 = 42;
    ctx.get_state().unwrap().set(SHARED_KEY, &value.to_le_bytes());
    value as i64
}
