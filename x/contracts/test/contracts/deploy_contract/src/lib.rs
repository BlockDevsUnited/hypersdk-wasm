// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

extern crate alloc;
use alloc::vec::Vec;
use wasmlanche::{Context, types::{ContractId}};
use wasmlanche::borsh::{BorshDeserialize, BorshSerialize};

// Import set_call_result and trace from the env module
extern "C" {
    fn set_call_result(ptr: *const u8, len: usize);
    fn trace(ptr: *const u8, len: usize) -> ();
    fn store_state(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
}

// Helper function to trace messages (for cleaner code)
unsafe fn trace_debug(message: &str) {
    let bytes = message.as_bytes();
    trace(bytes.as_ptr(), bytes.len());
}

// First, implement the C-style export for wasm_deploy
#[no_mangle]
pub extern "C" fn wasm_deploy(args: *const u8) -> u32 {
    // Right now we're just using the args pointer as a placeholder
    // In a real implementation we would extract the contract ID from args
    let _ = args;
    
    // Create a context for execution
    let mut ctx = Context::default();
    
    // Call the implementation function with a placeholder contract ID
    // In a real implementation we would extract this from args
    let contract_id = ContractId::new([0u8; 32]);
    let result = deploy_impl(&mut ctx, contract_id);
    
    // Convert the result to a heap-allocated pointer
    // This would be used for C-style return values
    let boxed = alloc::boxed::Box::new(result);
    let ptr = alloc::boxed::Box::into_raw(boxed);
    
    // Return the pointer
    ptr as u32
}

// Implement the Go test expected "deploy" function - THIS MUST BE THE EXPORT NAME!
#[cfg(target_arch = "wasm32")]
#[export_name = "deploy"]
pub extern "C" fn deploy(params_offset: i32) -> i32 {
    unsafe {
        trace_debug("==========================================");
        trace_debug("ENTERING DEPLOY FUNCTION");
        trace_debug("==========================================");
    }
    
    // Parse input parameters to get the target contract ID
    let params = read_input_params(params_offset);
    
    unsafe {
        trace_debug(&alloc::format!("ENTERING DEPLOY: RECEIVED PARAMS: {} bytes", params.len()));
        // Dump params bytes for debugging
        if !params.is_empty() {
            let mut param_bytes_str = String::new();
            for (i, &b) in params.iter().enumerate() {
                param_bytes_str.push_str(&alloc::format!("{:02x}", b));
                if i < params.len() - 1 {
                    param_bytes_str.push_str(" ");
                }
                if i > 20 {
                    param_bytes_str.push_str("...");
                    break;
                }
            }
            trace_debug(&alloc::format!("PARAMS BYTES: {}", param_bytes_str));
        } else {
            trace_debug("PARAMS EMPTY!");
        }
    }
    
    // Validate parameters (should be 32 bytes for contract ID)
    if params.len() != 32 {
        unsafe {
            trace_debug(&alloc::format!("ERROR: Invalid params length: {}, expected 32", params.len()));
            // Return empty address as error
            let empty_address = [0u8; 33];
            set_call_result(empty_address.as_ptr(), empty_address.len());
        }
        return 0;
    }
    
    unsafe {
        // Create a fixed key for storage - use a constant byte array to avoid any allocation issues
        let storage_key = b"deplcontract";
        trace_debug(&alloc::format!("Storage key: {:?} (length: {})", storage_key, storage_key.len()));
        
        // Store the contract ID as value - copy bytes from params directly with additional safeguards
        let mut contract_id_bytes = [0u8; 32];
        let copy_len = core::cmp::min(params.len(), 32);
        trace_debug(&alloc::format!("Copying {} bytes from params to contract_id_bytes", copy_len));
        
        for i in 0..copy_len {
            contract_id_bytes[i] = params[i];
        }
        
        trace_debug(&alloc::format!("CONTRACT ID BYTES: {:?}", contract_id_bytes));
        
        // IMPORTANT: Call store_state with our fixed key and the contract ID as value
        let store_result = store_state(
            storage_key.as_ptr(),
            storage_key.len(),
            contract_id_bytes.as_ptr(),
            contract_id_bytes.len()
        );
        
        trace_debug(&alloc::format!("STORE_STATE RESULT: {}", store_result));
        
        // Create address for return (33 bytes: type ID + contract ID)
        // Format: [type_id (0)] + [contract_id (32 bytes)]
        let mut address = [0u8; 33];
        address[0] = 0; // Type ID for contracts is 0
        for i in 0..32 {
            address[i+1] = contract_id_bytes[i];
        }
        
        trace_debug(&alloc::format!("RETURNING ADDRESS: {:x?} (length: {})", address, address.len()));
        
        // Set the result to be returned to the caller
        set_call_result(address.as_ptr(), address.len());
    }
    
    // Return success
    0
}

// Helper function to read input parameters
fn read_input_params(offset: i32) -> Vec<u8> {
    // First, we need to read the length of input (this is a convention in WebAssembly)
    let mut len_bytes = [0u8; 4];
    unsafe {
        trace_debug(&alloc::format!("READING INPUT PARAMS FROM OFFSET: {}", offset));
        
        if let Some(ptr) = core::ptr::NonNull::new(offset as *mut u8) {
            // Read 4 bytes for the length
            core::ptr::copy_nonoverlapping(ptr.as_ptr(), len_bytes.as_mut_ptr(), 4);
            trace_debug(&alloc::format!("READ LENGTH BYTES: {:02x} {:02x} {:02x} {:02x}", 
                        len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]));
        } else {
            trace_debug("ERROR: Invalid offset pointer");
            return Vec::new();
        }
    }
    
    // Convert 4 bytes to u32 (length)
    let len = u32::from_le_bytes(len_bytes) as usize;
    unsafe { trace_debug(&alloc::format!("DECODED LENGTH: {}", len)); }
    
    if len == 0 {
        unsafe { trace_debug("WARNING: Read zero length from input parameters"); }
        return Vec::new();
    }
    
    // Check if length seems reasonable - guard against invalid lengths
    if len > 1024 {
        unsafe { trace_debug(&alloc::format!("ERROR: Unreasonable parameter length: {}", len)); }
        return Vec::new();
    }
    
    // Read the parameters
    let mut params = Vec::with_capacity(len);
    unsafe {
        trace_debug(&alloc::format!("READING {} BYTES FROM OFFSET {}", len, offset));
        
        if let Some(ptr) = core::ptr::NonNull::new((offset + 4) as *mut u8) {
            // Allocate space
            params.resize(len, 0);
            // Read bytes safely
            core::ptr::copy_nonoverlapping(ptr.as_ptr(), params.as_mut_ptr(), len);
            trace_debug("SUCCESSFULLY READ PARAMETER BYTES");
        } else {
            trace_debug("ERROR: Invalid data pointer");
        }
    }
    
    params
}

// Implementation function that is not directly exported
fn deploy_impl(_ctx: &mut Context, contract_id: ContractId) -> Vec<u8> {
    // Create a properly formatted address: type (0) + contract_id
    let mut address = Vec::with_capacity(33);
    address.push(0); // Type ID for contracts is 0
    address.extend_from_slice(contract_id.as_bytes());
    address
}
