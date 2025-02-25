// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    mem,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};
use sdk_macros::public;
use wasmlanche::Context;

#[derive(Default)]
struct FixedAlloc {
    data: UnsafeCell<Vec<u8>>,
    size: AtomicUsize,
}

unsafe impl Sync for FixedAlloc {}

impl FixedAlloc {
    const fn new() -> Self {
        Self {
            data: UnsafeCell::new(Vec::new()),
            size: AtomicUsize::new(0),
        }
    }
}

unsafe impl GlobalAlloc for FixedAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let data = &mut *self.data.get();
        let offset = self.size.load(Ordering::Relaxed);
        let padding = offset % layout.align();
        let new_offset = offset + padding;
        data.resize(new_offset + layout.size(), 0);
        self.size.store(new_offset + layout.size(), Ordering::Relaxed);
        data.as_mut_ptr().add(new_offset)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Memory is freed when FixedAlloc is dropped
    }
}

#[global_allocator]
static ALLOC: FixedAlloc = FixedAlloc::new();

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
    let addr = context.actor.as_bytes();
    addr.iter()
        .map(|byte| *byte as u32)
        .fold(0, |acc, byte| (acc << 1) + (byte & 1))
}

#[public]
pub fn allocate_context(_: &mut Context) -> u32 {
    let layout = Layout::from_size_align(mem::size_of::<Context>(), 8).unwrap();
    let ptr = unsafe { ALLOC.alloc(layout) };
    if ptr.is_null() {
        panic!("failed to allocate memory");
    }
    ptr as u32
}

#[public]
pub fn allocate(_context: &mut Context, data: &[u8]) -> u32 {
    let layout = Layout::from_size_align(data.len(), 8).unwrap();
    let ptr = unsafe { ALLOC.alloc(layout) };
    if ptr.is_null() {
        panic!("failed to allocate memory");
    }
    unsafe {
        ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    }
    ptr as u32
}

#[public]
pub fn test_allocation(_context: &mut Context) -> Vec<u8> {
    let layout = Layout::from_size_align(mem::size_of::<Context>(), 8).unwrap();
    let ptr = unsafe { ALLOC.alloc(layout) };
    let data = b"test".to_vec();
    unsafe {
        ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    }
    data
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
}
