// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![no_std]
extern crate alloc;

use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::types::WasmlAddress;
use wasmlanche::Context;
use sdk_macros::public;
use alloc::string::String;

// This struct must match the Go ComplexReturn struct
#[derive(BorshSerialize, BorshDeserialize)]
pub struct ComplexReturn {
    Contract: WasmlAddress,
    MaxUnits: u64,
}

#[public]
pub fn get_value(ctx: &mut Context, _op_id: String) -> ComplexReturn {
    // Added a second parameter to maintain compatibility with the test
    // This parameter is not actually needed in the synchronous version
    ComplexReturn {
        Contract: ctx.actor.clone(),
        MaxUnits: 1000,
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
