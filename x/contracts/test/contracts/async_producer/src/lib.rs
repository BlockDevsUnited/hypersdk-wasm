// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![no_std]
extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use borsh::{BorshDeserialize, BorshSerialize};
use sdk_macros::public;
use wasmlanche::Context;

const SHARED_KEY: &[u8] = b"shared_value";

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

#[public]
pub fn check_operation(ctx: &mut Context, op_id: String) -> bool {
    // Check if an async operation has completed
    ctx.check_async_operation(&op_id)
}

#[public]
pub fn get_operation_result(ctx: &mut Context, op_id: String) -> i64 {
    // Get the result of a completed async operation
    // Returns 42 if successful, negative value if error
    if !ctx.check_async_operation(&op_id) {
        return -1; // Operation not complete
    }
    
    // For this example, we're not really checking the result
    // Just returning a success value
    42
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
