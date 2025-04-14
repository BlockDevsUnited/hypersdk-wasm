// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

// In a `no_std` environment, we need to bring our own allocator.
// Here, we're using wee_alloc.
#[cfg(target_arch = "wasm32")]
extern crate alloc;

#[cfg(target_arch = "wasm32")]
use wee_alloc;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// We also need to provide a panic handler
#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
