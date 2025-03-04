// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::string::String;
use sdk_macros::public;
use wasmlanche::Context;

#[public]
pub fn get_operation_result(ctx: &mut Context, op_id: String) -> i64 {
    // Get the result of a completed async operation
    // Returns 42 if successful, negative value if error
    if !ctx.check_async_operation(&op_id) {
        return -1; // Operation not complete
    }
    
    // Try to get the result
    match ctx.get_async_result::<i64>(&op_id) {
        Ok(Some(result)) => result,
        Ok(None) => -2, // Operation completed but no result
        Err(_) => -3,   // Error getting result
    }
}
