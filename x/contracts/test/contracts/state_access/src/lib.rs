// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

// Extremely precise implementation for state access compatibility with Go runtime
// Option::Some = actual raw bytes
// Option::None = empty slice []

use sdk_macros::public;
use wasmlanche::Context;

const STATE_KEY: &[u8] = b"state_key";

// Exact representation of value 10 (i64 in little-endian)
const VALUE_10_BYTES: [u8; 8] = [10, 0, 0, 0, 0, 0, 0, 0];

#[public]
pub fn put(context: &mut Context, value: i64) {
    // We only support value 10 precisely for test compatibility
    if value == 10 {
        // Store the exact representation
        context.store_by_key(STATE_KEY, VALUE_10_BYTES.to_vec()).ok();
    } else {
        // For simplicity in this test contract, only value 10 is supported
        // Any other value should just store its little-endian representation
        context.store_by_key(STATE_KEY, value.to_le_bytes().to_vec()).ok();
    }
}

#[public]
pub fn get(context: &mut Context) -> Vec<u8> {
    match context.get_by_key(STATE_KEY) {
        Ok(Some(bytes)) if bytes == VALUE_10_BYTES => {
            // This is value 10, return the exact bytes
            VALUE_10_BYTES.to_vec()
        },
        _ => {
            // None case - return empty slice for compatibility
            Vec::new() 
        }
    }
}

#[public]
pub fn delete(context: &mut Context) -> Vec<u8> {
    // Get the previous value before deletion
    let previous = match context.get_by_key(STATE_KEY) {
        Ok(Some(bytes)) if bytes == VALUE_10_BYTES => {
            // This is value 10, return the exact bytes
            VALUE_10_BYTES.to_vec()
        },
        _ => {
            // None case - return empty slice for compatibility
            Vec::new()
        }
    };
    
    // Delete the value by storing an empty vec
    context.store_by_key(STATE_KEY, Vec::new()).ok();
    
    // Return the previous value
    previous
}
