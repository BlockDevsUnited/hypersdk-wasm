// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone)]
struct AsyncCallArgs {
    pub target_contract: Vec<u8>,  // Address of the target contract
    pub method_name: String,       // Method to call
    pub args: Vec<u8>,             // Arguments to pass
    pub timeout_ms: u64,           // Timeout in milliseconds
    pub callback_id: String,       // Callback ID for the response
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone)]
struct AsyncCallResult {
    pub success: bool,
    pub return_data: Vec<u8>,
    pub callback_id: String,
    pub timestamp: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone, Default)]
struct AsyncState {
    pub pending_calls: HashMap<String, AsyncCallArgs>,
    pub completed_calls: HashMap<String, AsyncCallResult>,
    pub error_log: Vec<String>,
    pub call_counter: u64,
    pub events: Vec<String>,
}

pub struct AsyncContractExample {
    async_state: RwLock<AsyncState>,
}

impl AsyncContractExample {
    pub fn new() -> Self {
        Self {
            async_state: RwLock::new(AsyncState::default()),
        }
    }

    /// Make an asynchronous call to another contract
    pub fn call_other_contract(
        &self,
        target_contract: Vec<u8>,
        method_name: String,
        args: Vec<u8>,
        timeout_ms: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Generate a unique callback ID
        let mut state = self.async_state.write().unwrap();
        state.call_counter += 1;
        let callback_id = format!("callback_{}", state.call_counter);
        
        // Create the call arguments
        let call_args = AsyncCallArgs {
            target_contract,
            method_name,
            args,
            timeout_ms,
            callback_id: callback_id.clone(),
        };
        
        // Store in pending calls
        state.pending_calls.insert(callback_id.clone(), call_args);
        
        // Log this event
        self.log_event(&format!("Initiated async call with ID: {}", callback_id))?;
        
        Ok(callback_id)
    }

    /// Process an asynchronous result value when it's returned
    pub fn process_async_value(
        &self,
        callback_id: String,
        success: bool,
        return_data: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.async_state.write().unwrap();
        
        // Verify this is a pending call
        if !state.pending_calls.contains_key(&callback_id) {
            let error_msg = format!("Callback ID '{}' is not a pending call", callback_id);
            state.error_log.push(error_msg.clone());
            return Err(error_msg.into());
        }
        
        // Remove from pending calls
        let call_args = state.pending_calls.remove(&callback_id).unwrap();
        
        // Create the result
        let call_result = AsyncCallResult {
            success,
            return_data,
            callback_id: callback_id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        // Store in completed calls
        state.completed_calls.insert(callback_id.clone(), call_result);
        
        // Log the event
        let event_msg = if success {
            format!("Async call {} completed successfully", callback_id)
        } else {
            format!("Async call {} completed with failure", callback_id)
        };
        
        // Replace with direct logging since we have the lock already
        state.events.push(event_msg);
        
        Ok(())
    }

    /// Check if an asynchronous result is available
    pub fn has_async_result(&self, callback_id: &str) -> bool {
        let state = self.async_state.read().unwrap();
        state.completed_calls.contains_key(callback_id)
    }

    /// Get an asynchronous result value if available
    pub fn get_async_result(
        &self,
        callback_id: &str,
    ) -> Result<Option<AsyncCallResult>, Box<dyn std::error::Error>> {
        let state = self.async_state.read().unwrap();
        
        // Check if it's still pending
        if state.pending_calls.contains_key(callback_id) {
            return Ok(None); // Still pending
        }
        
        // Check if it's in completed calls
        match state.completed_calls.get(callback_id) {
            Some(result) => Ok(Some(result.clone())),
            None => {
                // Not found in either pending or completed
                Err(format!("No async call with ID '{}' found", callback_id).into())
            }
        }
    }

    /// Clear completed async results to prevent memory bloat
    pub fn clear_completed_results(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let mut state = self.async_state.write().unwrap();
        let completed_count = state.completed_calls.len() as u64;
        state.completed_calls.clear();
        
        // Log this event
        let event_msg = format!("Cleared {} completed async results", completed_count);
        state.events.push(event_msg);
        
        Ok(completed_count)
    }

    /// Get all pending callback IDs
    pub fn get_pending_callbacks(&self) -> Vec<String> {
        let state = self.async_state.read().unwrap();
        state.pending_calls.keys().cloned().collect()
    }

    /// Get all completed callback IDs
    pub fn get_completed_callbacks(&self) -> Vec<String> {
        let state = self.async_state.read().unwrap();
        state.completed_calls.keys().cloned().collect()
    }

    /// Log an event
    pub fn log_event(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.async_state.write().unwrap();
        state.events.push(message.to_string());
        Ok(())
    }

    /// Get all logged events
    pub fn get_events(&self) -> Vec<String> {
        let state = self.async_state.read().unwrap();
        state.events.clone()
    }

    /// Get all error logs
    pub fn get_error_logs(&self) -> Vec<String> {
        let state = self.async_state.read().unwrap();
        state.error_log.clone()
    }

    /// Async version of call_other_contract that returns a Future
    pub fn call_other_contract_async(
        &self,
        target_contract: Vec<u8>,
        method_name: String,
        args: Vec<u8>,
        timeout_ms: u64,
    ) -> Pin<Box<dyn Future<Output = Result<AsyncCallResult, String>> + Send>> {
        // First, make the normal call to get the callback ID
        let callback_id = match self.call_other_contract(target_contract, method_name, args, timeout_ms) {
            Ok(id) => id,
            Err(e) => {
                // Return early with the error message
                let error_msg = format!("Error: {}", e);
                return Box::pin(async move { Err(error_msg) });
            }
        };
        
        // Clone self for the async block
        let this = Arc::new(self.clone());
        let callback_id_clone = callback_id.clone();
        
        // Return a future that waits for the result
        Box::pin(async move {
            // In a real implementation, this would use actual async waiting
            // For demonstration, we'll simulate immediate completion
            
            // Create a simulated result
            let simulated_result = AsyncCallResult {
                success: true,
                return_data: vec![1, 2, 3, 4],
                callback_id: callback_id_clone,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            
            // In a real implementation, you would:
            // 1. Wait for the result to be available
            // 2. Poll the result status or use a notification mechanism
            // 3. Return the result when available or timeout
            
            Ok(simulated_result)
        })
    }
}

// Allow cloning for use in async contexts
impl Clone for AsyncContractExample {
    fn clone(&self) -> Self {
        // Clone by creating a new instance that shares the same state
        let async_state = self.async_state.read().unwrap().clone();
        let new_instance = Self {
            async_state: RwLock::new(async_state),
        };
        new_instance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_call_flow() {
        let contract = AsyncContractExample::new();
        
        // Make an async call
        let callback_id = contract
            .call_other_contract(
                vec![1, 2, 3, 4], // Target contract
                "test_method".to_string(),
                vec![5, 6, 7, 8], // Arguments
                1000, // Timeout
            )
            .expect("Failed to make async call");
        
        // Verify it's pending
        assert!(contract.get_pending_callbacks().contains(&callback_id));
        assert!(!contract.has_async_result(&callback_id));
        
        // Process the result
        contract
            .process_async_value(
                callback_id.clone(), 
                true, // Success
                vec![10, 11, 12, 13], // Return data
            )
            .expect("Failed to process async value");
        
        // Verify it's completed
        assert!(!contract.get_pending_callbacks().contains(&callback_id));
        assert!(contract.get_completed_callbacks().contains(&callback_id));
        assert!(contract.has_async_result(&callback_id));
        
        // Get the result
        let result = contract
            .get_async_result(&callback_id)
            .expect("Failed to get async result")
            .expect("No result found");
        
        // Verify the result
        assert_eq!(result.success, true);
        assert_eq!(result.return_data, vec![10, 11, 12, 13]);
        assert_eq!(result.callback_id, callback_id);
    }

    #[test]
    fn test_error_handling() {
        let contract = AsyncContractExample::new();
        
        // Try to get a non-existent result
        let result = contract.get_async_result("non_existent_id");
        assert!(result.is_err());
        
        // Try to process a non-existent callback
        let process_result = contract.process_async_value(
            "non_existent_id".to_string(),
            true,
            vec![1, 2, 3],
        );
        assert!(process_result.is_err());
        
        // Verify the error was logged
        let error_logs = contract.get_error_logs();
        assert_eq!(error_logs.len(), 1);
        assert!(error_logs[0].contains("non_existent_id"));
    }

    #[test]
    fn test_clear_completed() {
        let contract = AsyncContractExample::new();
        
        // Make several async calls
        let mut callback_ids = Vec::new();
        for i in 0..5 {
            let callback_id = contract
                .call_other_contract(
                    vec![i], // Target contract
                    format!("method_{}", i),
                    vec![i], // Arguments
                    1000, // Timeout
                )
                .expect("Failed to make async call");
            
            callback_ids.push(callback_id);
        }
        
        // Process all results
        for callback_id in &callback_ids {
            contract
                .process_async_value(
                    callback_id.clone(), 
                    true, 
                    vec![42], 
                )
                .expect("Failed to process async value");
        }
        
        // Verify all are completed
        assert_eq!(contract.get_completed_callbacks().len(), 5);
        
        // Clear completed
        let cleared = contract.clear_completed_results().expect("Failed to clear");
        assert_eq!(cleared, 5);
        
        // Verify all are cleared
        assert_eq!(contract.get_completed_callbacks().len(), 0);
    }
}

fn main() {
    println!("Async Contract Example");
    println!("----------------------");
    println!("This example demonstrates how to implement async patterns in a contract.");
    println!("Run the tests with 'cargo test --example async_contract_example' to see it in action.");
    
    // We could add code here to demonstrate the contract, but for now just show how to run the tests
}
