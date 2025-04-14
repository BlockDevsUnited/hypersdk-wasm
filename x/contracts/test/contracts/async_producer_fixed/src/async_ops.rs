// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::string::String;
use alloc::format;
use sdk_macros::public;
use wasmlanche::Context;
use crate::SHARED_KEY;

#[public]
pub async fn produce_async(ctx: &mut Context) -> i64 {
    // Produce a value asynchronously
    let value: i32 = 1337;
    let bytes = value.to_le_bytes().to_vec();
    
    match ctx.store_by_key_async(SHARED_KEY, bytes).await {
        Ok(_) => value as i64,
        Err(_) => -1, // Error code if storage fails
    }
}

#[public]
pub fn get_operation_result(ctx: &mut Context, op_id: String) -> i64 {
    // First check if the operation is complete
    match ctx.check_async_operation(&op_id) {
        Ok(wasmlanche::AsyncStatus::Completed) => {
            // Try to get the result
            match ctx.get_async_result::<i64>(&op_id) {
                Ok(Some(result)) => result,
                Ok(None) => -1, // Operation completed but no result
                Err(_) => -2,   // Error getting result
            }
        },
        Ok(wasmlanche::AsyncStatus::Pending) => {
            -3 // Operation still pending
        },
        Ok(wasmlanche::AsyncStatus::Failed) => {
            -4 // Operation failed
        },
        Err(_) => {
            -5 // Error checking operation status
        },
    }
}
