#![no_std]
extern crate alloc;

// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::string::String;
use alloc::format;
use sdk_macros::public;
use wasmlanche::Context;

// This is the shared key we'll use for all operations
pub const SHARED_KEY: &[u8] = b"shared_value";

// Include our modules with separate #[public] functions
pub mod async_ops;
pub mod cross_contract;

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

// Common panic handler
#[panic_handler]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
