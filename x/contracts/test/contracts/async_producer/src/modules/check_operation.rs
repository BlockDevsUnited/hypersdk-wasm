// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::string::String;
use sdk_macros::public;
use wasmlanche::Context;

#[public]
pub fn check_operation(ctx: &mut Context, op_id: String) -> bool {
    // Check if an async operation has completed
    ctx.check_async_operation(&op_id)
}
