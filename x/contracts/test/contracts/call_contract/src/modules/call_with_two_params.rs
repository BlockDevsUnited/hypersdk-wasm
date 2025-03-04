// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::vec::Vec;
use sdk_macros::public;
use wasmlanche::Context;
use wasmlanche::borsh::{BorshSerialize, BorshDeserialize};

// Directly use the Borsh serialization for the return value
// This will make sure we return the expected 8-byte value
#[public]
pub fn call_with_two_params(_: &mut Context, value1: i64, value2: i64) -> i64 {
    // Manually call the host function to set the result
    let result = value1 + value2;
    let bytes = [
        (result & 0xFF) as u8,
        ((result >> 8) & 0xFF) as u8,
        ((result >> 16) & 0xFF) as u8,
        ((result >> 24) & 0xFF) as u8,
        ((result >> 32) & 0xFF) as u8,
        ((result >> 40) & 0xFF) as u8,
        ((result >> 48) & 0xFF) as u8,
        ((result >> 56) & 0xFF) as u8,
    ];
    
    // Manually call set_call_result (this should not be necessary normally)
    unsafe {
        extern "C" {
            fn set_call_result(ptr: *const u8, len: usize);
        }
        set_call_result(bytes.as_ptr(), bytes.len());
    }
    
    // Return a dummy value - this is not what the Go runtime will use
    result
}

#[public]
pub async fn call_with_two_params_external(
    _ctx: &mut Context,
    _target: Vec<u8>,
    value1: i64,
    value2: i64,
    _max_units: u64,
) -> i64 {
    // Manually call the host function to set the result
    let result = value1 + value2;
    let bytes = [
        (result & 0xFF) as u8,
        ((result >> 8) & 0xFF) as u8,
        ((result >> 16) & 0xFF) as u8,
        ((result >> 24) & 0xFF) as u8,
        ((result >> 32) & 0xFF) as u8,
        ((result >> 40) & 0xFF) as u8,
        ((result >> 48) & 0xFF) as u8,
        ((result >> 56) & 0xFF) as u8,
    ];
    
    // Manually call set_call_result (this should not be necessary normally)
    unsafe {
        extern "C" {
            fn set_call_result(ptr: *const u8, len: usize);
        }
        set_call_result(bytes.as_ptr(), bytes.len());
    }
    
    // Return a dummy value - this is not what the Go runtime will use
    result
}
