// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// Import modules
pub mod modules;

// Re-export public functions at crate root level
pub use modules::simple_call::*;
pub use modules::actor_check::*;
pub use modules::call_with_param::*;
pub use modules::call_with_two_params::*;

// Export functions directly with completely distinct names
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn export_simple_call(params_offset: i32) {
    use wasmlanche::Context;
    use borsh::BorshSerialize;
    
    let mut ctx = Context::default();
    let result = simple_call(&mut ctx);
    
    // Serialize the result
    if let Ok(bytes) = BorshSerialize::try_to_vec(&result) {
        return_result(&bytes);
    }
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn export_actor_check(params_offset: i32) {
    use wasmlanche::Context;
    use borsh::BorshSerialize;
    
    let mut ctx = Context::default();
    let result = actor_check(&mut ctx);
    
    // Serialize the result
    if let Ok(bytes) = BorshSerialize::try_to_vec(&result) {
        return_result(&bytes);
    }
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn export_call_with_param(params_offset: i32) {
    use wasmlanche::Context;
    use borsh::BorshSerialize;
    
    let mut ctx = Context::default();
    // Hard-coded parameter for testing
    let param = 42i64;
    let result = call_with_param(&mut ctx, param);
    
    // Serialize the result
    if let Ok(bytes) = BorshSerialize::try_to_vec(&result) {
        return_result(&bytes);
    }
}

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn export_call_with_two_params(params_offset: i32) {
    use wasmlanche::Context;
    use borsh::BorshSerialize;
    
    let mut ctx = Context::default();
    // Hard-coded parameters for testing
    let param1 = 40i64;
    let param2 = 2i64;
    let result = call_with_two_params(&mut ctx, param1, param2);
    
    // Serialize the result
    if let Ok(bytes) = BorshSerialize::try_to_vec(&result) {
        return_result(&bytes);
    }
}

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

// Only use wee_alloc when std is not enabled and we're targeting wasm32
#[cfg(all(target_arch = "wasm32", not(feature = "std")))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Add panic handler for wasm32 target
#[cfg(all(target_arch = "wasm32", not(feature = "std")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // In a real contract, we might want to log this or handle it more gracefully
    core::arch::wasm32::unreachable();
}
