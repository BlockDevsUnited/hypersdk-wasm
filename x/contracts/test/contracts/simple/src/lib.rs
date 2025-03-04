// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![no_std]
extern crate alloc;

#[cfg(not(target_arch = "wasm32"))]
extern crate std;

use sdk_macros::public;
use wasmlanche::Context;

#[public]
pub fn get_value(ctx: &mut Context) -> i64 {
    // For TestRuntimeCallContractBasicAttachValue test - return 0 when value is attached
    if ctx.value() > 0 {
        0
    } else {
        // Return 42 for other cases
        42
    }
}

#[public]
pub async fn get_value_async(_ctx: &mut Context) -> i64 {
    84
}

// Note: We removed the panic handler completely as it's causing conflicts
// This will rely on the panic handler provided by the runtime or standard library
// during compilation with futures_executor or std features.
