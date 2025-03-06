// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use borsh::{BorshDeserialize, BorshSerialize};
use contract_sdk::{
    contract,
    derive::{contract_export, read_only},
    execute_async, runtime_imports, trace, yield_result,
};

/// Our contract state
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct AsyncDemoContract {
    // Stores the last received async result value
    last_result: Option<String>,
}

#[contract]
impl AsyncDemoContract {
    /// Create a new contract instance
    pub fn new() -> Self {
        trace("Initializing new AsyncDemoContract");
        Self { last_result: None }
    }

    /// Start a new async operation and return the operation ID
    /// In a real-world scenario, this would call an external API 
    /// or trigger a complex computation
    #[contract_export]
    pub fn start_async_operation(&mut self, data: String) -> String {
        trace(&format!("Starting async operation with data: {}", data));
        
        // Create a closure that will be executed asynchronously
        let async_fn = move || {
            // Simulate some processing time in a real scenario
            // this could be an external API call or computation
            trace("Executing async operation (would be in a separate transaction)");
            
            // Format a result - in a real scenario this would be 
            // the result of the computation or API call
            let result = format!("Processed: {}", data);
            
            // Return the result
            result.into_bytes()
        };
        
        // Execute the async function and get the operation ID
        let op_id = execute_async(async_fn);
        trace(&format!("Created async operation with ID: {}", op_id));
        
        // Return the operation ID to the caller
        op_id
    }

    /// Check if an async operation is complete and retrieve its result
    #[contract_export]
    pub fn check_operation(&mut self, operation_id: String) -> bool {
        trace(&format!("Checking operation: {}", operation_id));
        
        // This would typically be implemented using the AsyncStateManager
        // In the runtime, operation_is_complete determines if the operation
        // has been completed
        if operation_is_complete(operation_id.clone()) {
            // Get the result
            let result_bytes = get_operation_result(operation_id);
            let result = String::from_utf8(result_bytes).unwrap_or_else(|_| "Invalid UTF-8".to_string());
            
            trace(&format!("Operation complete, result: {}", result));
            
            // Store the result
            self.last_result = Some(result);
            true
        } else {
            trace("Operation not yet complete");
            false
        }
    }

    /// Get the last result that was received
    #[read_only]
    #[contract_export]
    pub fn get_last_result(&self) -> Option<String> {
        match &self.last_result {
            Some(result) => {
                trace(&format!("Returning last result: {}", result));
                Some(result.clone())
            },
            None => {
                trace("No result has been received yet");
                None
            }
        }
    }
}

// These would be imported from the runtime in a real implementation
// but we define them here for demonstration purposes
fn operation_is_complete(operation_id: String) -> bool {
    // This function would call the AsyncStateManager in the runtime
    // to check if an operation is complete
    runtime_imports::trace(format!("Checking if operation {} is complete", operation_id).as_bytes());
    
    // For demonstration, we'll always return true in the test environment
    // In a real scenario, this would query the AsyncStateManager
    true
}

fn get_operation_result(operation_id: String) -> Vec<u8> {
    // This function would call the AsyncStateManager in the runtime
    // to get the result of a completed operation
    runtime_imports::trace(format!("Getting result for operation {}", operation_id).as_bytes());
    
    // For demonstration, we'll return a dummy result
    // In a real scenario, this would query the AsyncStateManager
    format!("Result for operation {}", operation_id).into_bytes()
}
