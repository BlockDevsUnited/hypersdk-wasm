// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::UnsafeCell,
};
use sdk_macros::public;
use wasmlanche::Context;

struct HighestAllocatedAddress {
    value: UnsafeCell<usize>,
}

unsafe impl Sync for HighestAllocatedAddress {}

static HIGHEST_ALLOCATED_ADDRESS: HighestAllocatedAddress = HighestAllocatedAddress {
    value: UnsafeCell::new(0),
};

struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);

        if ptr.is_null() {
            return ptr;
        }

        let addr = ptr as usize;
        let highest = HIGHEST_ALLOCATED_ADDRESS.value.get();

        if addr + layout.size() > *highest {
            *highest = addr + layout.size();
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

#[public]
pub fn highest_allocated_address(_: &mut Context) -> usize {
    unsafe { *HIGHEST_ALLOCATED_ADDRESS.value.get() }
}

#[public]
pub fn always_true(_: &mut Context) -> bool {
    true
}

#[public]
pub fn combine_last_bit_of_each_id_byte(context: &mut Context) -> u32 {
    let addr = context.actor().as_bytes();
    addr.iter()
        .map(|byte| *byte as u32)
        .fold(0, |acc, byte| (acc << 1) + (byte & 1))
}

#[public]
pub fn allocate_context(_: &mut Context) -> u32 {
    let layout = Layout::from_size_align(std::mem::size_of::<Context>(), 8).unwrap();
    let ptr = unsafe { GLOBAL.alloc(layout) };
    if ptr.is_null() {
        panic!("failed to allocate memory");
    }
    ptr as u32
}

#[public]
pub fn allocate(_context: &mut Context, data: &[u8]) -> u32 {
    let layout = Layout::from_size_align(data.len(), 8).unwrap();
    let ptr = unsafe { GLOBAL.alloc(layout) };
    if ptr.is_null() {
        panic!("failed to allocate memory");
    }
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    }
    ptr as u32
}

#[cfg(test)]
mod tests {
    use wasmlanche::{Address, Context};

    #[test]
    fn test_balance() {
        let address = Address::default();
        let mut context = Context::with_actor(address);
        let amount: u64 = 100;

        // set the balance
        context.mock_set_balance(address, amount);

        let balance = context.get_balance(address);
        assert_eq!(balance, amount);
    }
}
