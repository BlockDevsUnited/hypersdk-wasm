// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use alloc::vec::Vec;
use core::alloc::Layout;
use core::{mem, ptr};
use sdk_macros::public;
use wasmlanche::{Context, types::WasmlAddress};
use borsh::{BorshDeserialize, BorshSerialize};

#[public]
pub fn highest_allocated_address(_: &mut Context) -> usize {
    0
}

#[public]
pub fn always_true(_: &mut Context) -> bool {
    true
}

#[public]
pub fn combine_last_bit_of_each_id_byte(context: &mut Context) -> u32 {
    let id = context.actor.as_bytes();
    let mut result = 0u32;
    for (i, byte) in id.iter().enumerate() {
        result |= ((byte & 1) as u32) << i;
    }
    result
}

#[public]
pub fn test_balance(context: &mut Context) {
    let address = context.actor;
    let amount: u64 = 100;

    // Set balance and verify
    context.set_balance(&address, amount);
    let balance = context.get_balance(&address);
    assert_eq!(balance, amount);
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ComplexReturn {
    pub value: u32,
    pub data: Vec<u8>,
}

#[public]
pub fn get_value(context: &mut Context) -> ComplexReturn {
    ComplexReturn {
        value: combine_last_bit_of_each_id_byte(context),
        data: vec![1, 2, 3, 4, 5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmlanche::types::WasmlAddress;

    #[test]
    fn test_balance() {
        let address = WasmlAddress::new([0; 32]);
        let mut context = Context::with_actor(address.clone());
        let amount: u64 = 100;

        // Set balance and verify
        context.set_balance(&address, amount);
        let balance = context.get_balance(&address);
        assert_eq!(balance, amount);
    }

    #[test]
    fn test_complex_return() {
        let address = WasmlAddress::new([0; 32]);
        let mut context = Context::with_actor(address);
        let result = get_value(&mut context);
        assert_eq!(result.data, vec![1, 2, 3, 4, 5]);
    }
}
