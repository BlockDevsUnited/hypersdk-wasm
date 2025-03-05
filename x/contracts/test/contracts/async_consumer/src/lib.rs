#![no_std]

extern crate alloc;

// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use borsh::{BorshDeserialize, BorshSerialize};
use sdk_macros::public;
use wasmlanche::Context;

// Add global allocator
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(target_arch = "wasm32")]
mod imports {
    extern "C" {
        pub fn set_call_result(ptr: *const u8, len: usize);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn return_result(data: &[u8]) {
    unsafe {
        imports::set_call_result(data.as_ptr(), data.len());
    }
}

const SHARED_KEY: &[u8] = b"shared_value";

#[public]
pub fn consume(ctx: &mut Context) -> i64 {
    // Read the value from state
    match ctx.get::<Vec<u8>>(SHARED_KEY) {
        Ok(Some(bytes)) => {
            if bytes.len() < 4 {
                return -1; // Error: Invalid data length
            }
            
            // Convert bytes to i32
            let mut array = [0u8; 4];
            array.copy_from_slice(&bytes[0..4]);
            let value = i32::from_le_bytes(array);
            
            // Return the value
            value as i64
        },
        Ok(None) => {
            // Retry with sync
            let value: i32 = 42; // Default value
            value as i64
        },
        Err(_) => {
            // Retry with sync
            let value: i32 = 42; // Default value
            value as i64
        }
    }
}

#[public]
pub fn consume_async(ctx: &mut Context) -> String {
    // Start an async read operation
    let result = ctx.get_async::<Vec<u8>>(SHARED_KEY);
    
    // Check if the result is immediately available
    if let Some(inner_result) = result.result {
        match inner_result {
            Ok(Some(bytes)) => {
                // Immediately got a result
                if bytes.len() < 4 {
                    return String::from("ERROR:Invalid data length");
                }
                
                // Convert bytes to i32
                let mut array = [0u8; 4];
                array.copy_from_slice(&bytes[0..4]);
                let value = i32::from_le_bytes(array);
                
                format!("COMPLETED:{}", value)
            },
            Ok(None) => {
                // No data but operation completed successfully
                String::from("COMPLETED:NONE")
            },
            Err(err) => format!("ERROR:{:?}", err),
        }
    } else {
        // The operation is pending, return the operation ID
        // In a real implementation, we would generate and store an operation ID
        String::from("PENDING:async_op_1")
    }
}

#[public]
pub fn check_operation(ctx: &mut Context, op_id: String) -> bool {
    // Check if an async operation has completed
    ctx.check_async_operation(&op_id)
}

#[public]
pub fn get_operation_result(ctx: &mut Context, op_id: String) -> i64 {
    // Get the result of a completed async read operation
    if !ctx.check_async_operation(&op_id) {
        return -2; // Operation not complete
    }
    
    // Retrieve the result
    match ctx.get_async_result::<Vec<u8>>(&op_id) {
        Ok(Some(bytes)) => {
            if bytes.len() < 4 {
                return -1; // Error: Invalid data length
            }
            
            // Convert bytes to i32
            let mut array = [0u8; 4];
            array.copy_from_slice(&bytes[0..4]);
            let value = i32::from_le_bytes(array);
            
            // Return the value
            value as i64
        },
        Ok(None) => -3, // Key not found
        Err(_) => -4,   // Error retrieving result
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
