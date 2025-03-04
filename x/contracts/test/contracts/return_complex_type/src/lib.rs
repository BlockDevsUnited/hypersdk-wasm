#![no_std]
extern crate alloc;

// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::{Context, types::WasmlAddress};
use wasmlanche::borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ComplexReturn {
    account: WasmlAddress,
    max_units: u64,
}

#[public]
pub fn get_value(ctx: &mut Context) -> ComplexReturn {
    ComplexReturn {
        account: ctx.actor.clone(),
        max_units: 1000,
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
