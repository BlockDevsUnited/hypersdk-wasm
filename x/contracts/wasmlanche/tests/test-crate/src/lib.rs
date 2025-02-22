// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{string::String, vec, vec::Vec};
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    mem,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};
use sdk_macros::public;
use wasmlanche::{Context, Host, host::HostState, types::WasmlAddress};

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
pub fn allocate_context(_: &mut Context) -> i32 {
    let layout = Layout::from_size_align(mem::size_of::<Context>(), 8).unwrap();
    let ptr = unsafe { ALLOC.alloc(layout) };
    if ptr.is_null() {
        return -1; // Return error code instead of panicking
    }
    ptr as i32
}

#[public]
pub fn allocate(_context: &mut Context, data_ptr: i32, size: i32) -> i32 {
    if size <= 0 {
        return -1;
    }

    // Allocate new memory first
    let layout = Layout::from_size_align(size as usize, 8).unwrap();
    let new_ptr = unsafe { ALLOC.alloc(layout) };
    if new_ptr.is_null() {
        return -1;
    }

    // Copy data if source pointer is non-zero
    if data_ptr != 0 {
        // Check if source pointer is valid
        let source_end = data_ptr.checked_add(size)
            .ok_or_else(|| -1)
            .unwrap_or(-1);
        if source_end < 0 {
            unsafe { ALLOC.dealloc(new_ptr as *mut u8, layout) };
            return -1; // Use consistent error code
        }

        // Copy the data
        unsafe {
            // Get a slice of the source memory
            let source = core::slice::from_raw_parts(data_ptr as *const u8, size as usize);
            // Get a slice of the destination memory
            let dest = core::slice::from_raw_parts_mut(new_ptr, size as usize);
            // Copy the data
            dest.copy_from_slice(source);
        }
    } else {
        // Initialize memory to zero if no source data
        unsafe {
            core::ptr::write_bytes(new_ptr, 0, size as usize);
        }
    }

    // Return the pointer to the newly allocated memory
    new_ptr as i32
}

#[public]
pub fn test_allocation(context: &mut Context) -> Vec<u8> {
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
    use tokio::sync::RwLock;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_balance() {
        let address = WasmlAddress::new(vec![0; 33]);
        let mut context = Context::with_actor(address.clone());
        let amount: u64 = 100;

        // Set balance and verify
        context.set_balance(&address, amount);
        let balance = context.get_balance(&address);
        assert_eq!(balance, amount);
    }
}
