// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![no_std]
extern crate alloc;

#[cfg(not(target_arch = "wasm32"))]
extern crate std;

// Import modules
pub mod modules;

// Re-export public functions at crate root level
pub use modules::produce;
pub use modules::produce_async;
pub use modules::check_operation;
pub use modules::get_operation_result;

// Export the `SHARED_KEY` constant for use in modules
pub const SHARED_KEY: &[u8] = b"shared_value";

#[cfg(target_arch = "wasm32")]
mod imports {
    extern "C" {
        pub fn set_call_result(ptr: *const u8, len: usize);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn return_result(data: &[u8]) {
    unsafe {
        imports::set_call_result(data.as_ptr(), data.len());
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(not(target_arch = "wasm32"))]
pub static OPERATION_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
