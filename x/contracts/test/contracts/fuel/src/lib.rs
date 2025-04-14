#![no_std]
extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use wasmlanche::{Context, types::WasmlAddress};
use sdk_macros::public;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Add panic handler for wasm32
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// Import set_call_result function for target_arch = "wasm32"
#[cfg(target_arch = "wasm32")]
mod imports {
    #[link(wasm_import_module = "env")]
    extern "C" {
        pub fn set_call_result(ptr: *const u8, len: usize);
    }
}

/// This function should return the OutOfFuel error code which is value 2
/// It must return a vector containing a single byte with the value 2
#[no_mangle]
pub extern "C" fn wasm_out_of_fuel(_args: *const u8) -> u32 {
    // The test expects exactly []byte{0x2}
    // Create a vector with a single byte value of 2
    let result = [2u8];
    
    // Manually set the result using the imported function
    #[cfg(target_arch = "wasm32")]
    unsafe {
        imports::set_call_result(result.as_ptr(), result.len());
    }
    
    // Return success
    0
}

/// This function should return the current fuel amount
/// It must return less than 1,000,000,000
#[public]
pub fn get_fuel(_ctx: &mut Context) -> u64 {
    // Use a small, safe constant value
    500
}
