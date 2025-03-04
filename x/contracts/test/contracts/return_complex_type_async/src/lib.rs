#![no_std]
extern crate alloc;

// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::{Context, types::WasmlAddress};
use wasmlanche::borsh::{BorshSerialize, BorshDeserialize};
use alloc::string::String;
use alloc::format;

// This struct must match the Go ComplexReturn struct
#[derive(BorshSerialize, BorshDeserialize)]
pub struct ComplexReturn {
    Contract: WasmlAddress,
    MaxUnits: u64,
}

// The key used for storing our complex return value
const COMPLEX_VALUE_KEY: &[u8] = b"complex_value";

// Simplified version for testing compilation
#[public]
pub fn minimal_async_test(ctx: &mut Context) -> String {
    // Generate a unique operation ID
    match ctx.generate_operation_id() {
        Ok(op_id) => op_id,
        Err(_) => "ERROR:GenerationFailed".to_string(),
    }
}

#[public]
pub fn get_value_async(ctx: &mut Context) -> String {
    // Create the ComplexReturn object
    let value = ComplexReturn {
        Contract: ctx.actor.clone(),
        MaxUnits: 1000,
    };
    
    // Serialize the value
    if let Ok(bytes) = wasmlanche::borsh::to_vec(&value) {
        // Store the value asynchronously
        match ctx.store_by_key_async(COMPLEX_VALUE_KEY, bytes) {
            Ok(op_id) => op_id,
            Err(err) => format!("ERROR:{:?}", err),
        }
    } else {
        "ERROR:SerializationFailed".to_string()
    }
}

#[public]
pub fn get_value(ctx: &mut Context, op_id: String) -> ComplexReturn {
    // This function is needed to maintain compatibility with the original test
    // We'll just call get_complex_result directly with the operation ID provided
    get_complex_result(ctx, op_id)
}

#[public]
pub fn get_complex_result(ctx: &mut Context, op_id: String) -> ComplexReturn {
    // If operation is not complete, return default value
    if !ctx.check_async_operation(&op_id) {
        return ComplexReturn {
            Contract: ctx.actor.clone(),
            MaxUnits: 0,
        };
    }
    
    // Get value from storage
    match ctx.get_by_key(COMPLEX_VALUE_KEY) {
        Ok(Some(bytes)) => {
            // Try to deserialize
            if let Ok(value) = wasmlanche::borsh::from_slice::<ComplexReturn>(&bytes) {
                value
            } else {
                // Default value for deserialization error
                ComplexReturn {
                    Contract: ctx.actor.clone(),
                    MaxUnits: 0,
                }
            }
        },
        _ => ComplexReturn {
            Contract: ctx.actor.clone(),
            MaxUnits: 0,
        },
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
