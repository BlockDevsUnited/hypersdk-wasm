#![no_std]
extern crate alloc;

#[cfg(not(target_arch = "wasm32"))]
extern crate std;

use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::types::WasmlAddress;
use wasmlanche::Context;
use sdk_macros::public;
use alloc::vec::Vec;

// This struct must match the Go ComplexReturn struct
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct ComplexReturn {
    // Use field capitalization matching Go struct
    pub Contract: WasmlAddress,
    pub MaxUnits: u64,
}

#[public]
pub fn get_value(ctx: &mut Context) -> ComplexReturn {
    // Creating a value with a predictable pattern for debugging
    let mut address_bytes = [0u8; 32];
    for i in 0..32 {
        address_bytes[i] = i as u8;
    }
    
    ComplexReturn {
        Contract: WasmlAddress::new(address_bytes),
        MaxUnits: 1234,
    }
}

#[public]
pub fn get_serialized_value(_ctx: &mut Context) -> Vec<u8> {
    // Creating a value with a predictable pattern for debugging
    let mut address_bytes = [0u8; 32];
    for i in 0..32 {
        address_bytes[i] = i as u8;
    }
    
    let complex_return = ComplexReturn {
        Contract: WasmlAddress::new(address_bytes),
        MaxUnits: 1234,
    };
    
    // Serialize using borsh
    match borsh::to_vec(&complex_return) {
        Ok(bytes) => bytes,
        Err(_) => Vec::new(),
    }
}

#[public]
pub fn test_deserialize(_ctx: &mut Context, input: Vec<u8>) -> bool {
    // Try to deserialize the input and check if it matches expected
    match borsh::from_slice::<ComplexReturn>(&input) {
        Ok(value) => {
            // Create the expected value
            let mut address_bytes = [0u8; 32];
            for i in 0..32 {
                address_bytes[i] = i as u8;
            }
            
            let expected = ComplexReturn {
                Contract: WasmlAddress::new(address_bytes),
                MaxUnits: 1234,
            };
            
            value == expected
        },
        Err(_) => false,
    }
}

// Only define global allocator in a very specific case:
// 1. When targeting wasm32
// 2. When NOT using futures_executor feature
// 3. When NOT using the std feature
#[cfg(all(
    target_arch = "wasm32",
    not(feature = "futures_executor"),
    not(feature = "std")
))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Only define panic handler in a very specific case:
// 1. When targeting wasm32
// 2. When NOT using futures_executor feature
// 3. When NOT using the std feature
#[cfg(all(
    target_arch = "wasm32",
    not(feature = "futures_executor"),
    not(feature = "std")
))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
