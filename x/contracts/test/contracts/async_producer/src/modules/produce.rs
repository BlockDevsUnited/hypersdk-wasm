// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::vec::Vec;
use sdk_macros::public;
use wasmlanche::Context;
use crate::SHARED_KEY;

#[public]
pub fn produce(ctx: &mut Context) -> i64 {
    // Produce a value and store it in state
    let value: i32 = 42;
    let bytes = value.to_le_bytes().to_vec();
    
    match ctx.store_by_key(SHARED_KEY, bytes) {
        Ok(_) => value as i64,
        Err(_) => -1, // Error code if storage fails
    }
}
