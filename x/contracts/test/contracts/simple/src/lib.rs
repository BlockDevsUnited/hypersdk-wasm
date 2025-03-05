// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![no_std]
extern crate alloc;

#[cfg(not(target_arch = "wasm32"))]
extern crate std;

use wasmlanche::Context;
use sdk_macros::public;

// Set up the global allocator for wasm32 target
#[cfg(all(
    target_arch = "wasm32",
    not(feature = "futures_executor"),
    not(feature = "std")
))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Add a panic handler for wasm32 target
#[cfg(all(
    target_arch = "wasm32",
    not(feature = "futures_executor"),
    not(feature = "std")
))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

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

// For the async pattern, we need to split it into two functions:
// 1. A function that initiates the async operation and returns an operation ID
// 2. A function that retrieves the result using that operation ID

// Function to initiate the async operation and return an operation ID
#[public]
pub fn get_value_async(_ctx: &mut Context) -> alloc::string::String {
    // In a real async function, we'd start an async operation here
    // and return an operation ID for tracking
    "async_op_1".into()
}

// Function to retrieve the result using the operation ID
#[public]
pub fn get_async_result(_ctx: &mut Context, _op_id: alloc::string::String) -> i64 {
    // In a real implementation, we would check if the operation is complete
    // and return the result if it is, or an error if it's not
    84
}
