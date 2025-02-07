// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

//! Temporary storage allocated during the Contract runtime.
//! The general pattern for handling memory is to have the
//! host allocate a block of memory and return a pointer to
//! the contract. These methods are unsafe as should be used
//! with caution.

#[cfg(not(feature = "std"))]
use alloc::{
    alloc::{alloc as allocate, dealloc as deallocate, handle_alloc_error, Layout},
    string::String,
    vec::Vec,
};

#[cfg(feature = "std")]
use std::{
    alloc::{alloc as allocate, dealloc as deallocate, handle_alloc_error, Layout},
    string::String,
    vec::Vec,
};

use core::{mem::ManuallyDrop, ops::Deref, slice, fmt};

pub mod allocations;

/// A pointer to memory in the host environment.
#[derive(Debug)]
pub struct Memory {
    ptr: *mut u8,
    len: usize,
}

impl Memory {
    /// Create a new Memory from raw parts.
    pub fn from_raw_parts(ptr: *mut u8, len: usize) -> Self {
        Self { ptr, len }
    }

    /// Create a new Memory by allocating space for the given size.
    pub fn new(size: usize) -> Self {
        let layout = Layout::array::<u8>(size).unwrap();
        let ptr = unsafe { allocate(layout) };
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
        Self { ptr, len: size }
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get the raw mutable pointer.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    /// Get the length of the memory.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the memory is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            let layout = Layout::array::<u8>(self.len).unwrap();
            unsafe {
                deallocate(self.ptr, layout);
            }
        }
    }
}

impl Deref for Memory {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl AsRef<[u8]> for Memory {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

#[doc(hidden)]
/// A pointer where data points to the host.
#[cfg_attr(feature = "debug", derive(Debug))]
#[repr(transparent)]
pub struct HostPtr(*const u8);

impl Deref for HostPtr {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let len = allocations::get(self.0).expect("attempted to deref invalid host pointer");
        unsafe { slice::from_raw_parts(self.0, len) }
    }
}

impl Drop for HostPtr {
    fn drop(&mut self) {
        if self.is_null() {
            return;
        }

        let len = allocations::remove(self.0).expect("attempted to drop invalid host pointer");
        let layout = Layout::array::<u8>(len).expect("capacity overflow");

        unsafe { deallocate(self.0.cast_mut(), layout) };
    }
}

impl From<HostPtr> for Vec<u8> {
    fn from(host_ptr: HostPtr) -> Self {
        // drop will dealloc the bytes
        let host_ptr = ManuallyDrop::new(host_ptr);

        let len = allocations::remove(host_ptr.0)
            .expect("attempted to convert invalid host pointer to a Vec");

        unsafe { Vec::from_raw_parts(host_ptr.0.cast_mut(), len, len) }
    }
}

impl HostPtr {
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub fn null() -> Self {
        Self(std::ptr::null())
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.0
    }

    pub fn from_raw(ptr: *const u8) -> Self {
        Self(ptr)
    }
}

#[cfg(feature = "test")]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    #[should_panic(expected = "attempted to deref invalid host pointer")]
    fn deref_untracked_pointer() {
        let ptr = HostPtr(ptr::null());
        let _ = &*ptr;
    }

    #[test]
    #[should_panic(expected = "attempted to drop invalid host pointer")]
    fn drop_untracked_pointer() {
        let data = vec![0xff, 0xaa];
        let ptr = HostPtr(data.as_ptr());
        drop(ptr);
    }

    #[test]
    fn deref_tracked_pointer() {
        let data = vec![0xff];
        let cloned = data.clone();
        let data = ManuallyDrop::new(data);
        let ptr = data.as_ptr();

        allocations::insert(ptr, data.len());

        assert_eq!(&*HostPtr(ptr), &cloned);
    }

    #[test]
    fn deref_is_borrow() {
        let data = vec![0xff];
        let cloned = data.clone();
        let data = ManuallyDrop::new(data);
        let ptr = data.as_ptr();

        allocations::insert(ptr, data.len());

        let host_ptr = ManuallyDrop::new(HostPtr(ptr));
        assert_eq!(&**host_ptr, &cloned);
        let host_ptr = HostPtr(ptr);
        assert_eq!(&*host_ptr, &cloned);
    }

    #[test]
    fn host_pointer_to_vec_takes_bytes() {
        let data = vec![0xff];
        let cloned = data.clone();
        let data = ManuallyDrop::new(data);
        let ptr = data.as_ptr();

        allocations::insert(ptr, data.len());

        assert_eq!(Vec::from(HostPtr(ptr)), cloned);

        assert!(allocations::get(ptr).is_none());
    }

    #[test]
    #[should_panic(expected = "attempted to convert invalid host pointer to a Vec")]
    fn host_pointer_to_vec_panics_on_invalid_pointer() {
        let ptr = HostPtr(ptr::null());
        let _ = Vec::from(ptr);
    }

    #[test]
    fn dropping_host_pointer_deallocates() {
        let data = vec![0xff];
        let data = ManuallyDrop::new(data);
        let ptr = data.as_ptr();

        allocations::insert(ptr, data.len());

        drop(HostPtr(ptr));

        assert!(allocations::get(ptr).is_none());

        // overwrites old allocation
        let data = vec![0x00];
        assert_eq!(data.as_ptr(), ptr);
    }

    #[test]
    #[should_panic = "cannot allocate 0 sized data"]
    fn zero_allocation_panics() {
        alloc(0);
    }

    #[test]
    fn allocate_normal_length_data() {
        let len = 1024;
        let data: Vec<_> = (u8::MIN..=u8::MAX).cycle().take(len).collect();
        let ptr = alloc(len);

        unsafe { ptr::copy(data.as_ptr(), ptr.0.cast_mut(), data.len()) }

        assert_eq!(&*ptr, &*data);
    }
}

/// Allocate memory into the instance of Contract and return the offset to the
/// start of the block.
/// # Panics
/// Panics if the pointer exceeds the maximum size of an isize or that the allocated memory is null.
#[no_mangle]
pub(crate) extern "C-unwind" fn alloc(len: usize) -> HostPtr {
    assert!(len > 0, "cannot allocate 0 sized data");
    // can only fail if `len > isize::MAX` for u8
    let layout = Layout::array::<u8>(len).expect("capacity overflow");

    let ptr = unsafe { allocate(layout) };

    if ptr.is_null() {
        handle_alloc_error(layout);
    }

    allocations::insert(ptr, len);

    HostPtr(ptr.cast_const())
}
