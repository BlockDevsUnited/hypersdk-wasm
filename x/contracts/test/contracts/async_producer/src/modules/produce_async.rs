// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use sdk_macros::public;
use wasmlanche::Context;
use crate::SHARED_KEY;

#[public]
pub fn produce_async(ctx: &mut Context) -> String {
    // Produce a value and store it asynchronously in state
    let value: i32 = 42;
    let bytes = value.to_le_bytes().to_vec();
    
    // Call the async version of store_by_key via FFI
    // This function will return an operation ID
    match ctx.store_by_key_async(SHARED_KEY, bytes) {
        Ok(op_id) => op_id,
        Err(err) => format!("ERROR:{:?}", err),
    }
}
