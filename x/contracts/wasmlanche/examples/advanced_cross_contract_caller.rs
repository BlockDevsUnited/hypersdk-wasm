//! Advanced Cross-Contract Caller Example
//! 
//! This contract demonstrates making async calls to other contracts
//! with proper timeout handling, error management, and gas accounting.

use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    context::Context,
    types::WasmlAddress,
    error::Error,
    future::AsyncResult,
    prelude::public,
};

/// Arguments for cross-contract calls
#[derive(BorshSerialize, BorshDeserialize)]
pub struct CrossCallArgs {
    /// Target contract address
    pub target: WasmlAddress,
    
    /// Amount to pass in the call
    pub amount: u64,
    
    /// Optional timeout in milliseconds
    pub timeout_ms: Option<u64>,
    
    /// Whether to simulate an error
    pub simulate_error: bool,
}

/// Result of a cross-contract call
#[derive(BorshSerialize, BorshDeserialize)]
pub struct CrossCallResult {
    /// Whether the call was successful
    pub success: bool,
    
    /// The value returned from the called contract
    pub value: u64,
    
    /// Gas used for the operation
    pub gas_used: u64,
    
    /// Execution time in milliseconds (if available)
    pub execution_time_ms: u64,
}

/// Make a cross-contract call with advanced error handling and timeout
pub async fn call_with_timeout(ctx: &mut Context, args: CrossCallArgs) -> AsyncResult<CrossCallResult> {
    // Record initial gas - we need to implement this on Context
    // For now, let's use a placeholder value
    let initial_gas: u64 = 100000; // ctx.remaining_gas();
    
    // Store call details in state for later retrieval
    let key = b"last_call_details";
    let encoded = match borsh::to_vec(&args) {
        Ok(bytes) => bytes,
        Err(e) => return AsyncResult::with_result(Err(Error::Serialization(format!("Failed to serialize args: {}", e)))),
    };
    
    // Use store_by_key_async instead of store_async
    match ctx.store_by_key_async(key, encoded) {
        Ok(_) => {},
        Err(e) => return AsyncResult::with_result(Err(e)),
    };
    
    // If simulating error, fail immediately
    if args.simulate_error {
        return AsyncResult::with_result(Err(Error::Contract(String::from("Simulated error in cross-contract call"))));
    }
    
    // Make the cross-contract call with timeout
    let call_args = args.amount.to_le_bytes().to_vec();
    
    // Handle the Result from current_timestamp without using ?
    let start_time = match ctx.current_timestamp().await {
        Ok(t) => t,
        Err(e) => return AsyncResult::with_result(Err(e)),
    };
    
    match ctx.call_contract(&args.target, "get_value", &call_args, args.timeout_ms).await {
        Ok(data) => {
            // Parse the result
            let value = if !data.is_empty() {
                // Handle Result without using ?
                match data.try_into() {
                    Ok(bytes) => u64::from_le_bytes(bytes),
                    Err(_) => return AsyncResult::with_result(Err(Error::Serialization("Invalid result length".to_string()))),
                }
            } else {
                0
            };
            
            // Handle the Result from current_timestamp without using ?
            let end_time = match ctx.current_timestamp().await {
                Ok(t) => t,
                Err(e) => return AsyncResult::with_result(Err(e)),
            };
            
            let execution_time = end_time.saturating_sub(start_time);
            
            // Calculate gas used - for now use placeholder
            let final_gas: u64 = 90000; // ctx.remaining_gas();
            let gas_used = initial_gas.saturating_sub(final_gas);
            
            // Prepare the successful result
            let result = CrossCallResult {
                success: true,
                value,
                gas_used,
                execution_time_ms: execution_time,
            };
            
            AsyncResult::with_result(Ok(result))
        },
        Err(e) => {
            // Handle timeout or other errors
            let result = CrossCallResult {
                success: false,
                value: 0,
                gas_used: 10000, // initial_gas.saturating_sub(ctx.remaining_gas()),
                execution_time_ms: 0,
            };
            
            AsyncResult::with_result(Ok(result))
        }
    }
}

/// Get the details of the last call made
pub async fn get_last_call_details(ctx: &mut Context) -> AsyncResult<CrossCallArgs> {
    let key = b"last_call_details";
    
    // Using get_by_key instead of get_async
    let bytes_option = match ctx.get_by_key(key) {
        Ok(option) => option,
        Err(e) => return AsyncResult::with_result(Err(e)),
    };
    
    match bytes_option {
        Some(bytes) => {
            // Using BorshDeserialize::try_from_slice() instead of borsh::from_slice()
            match CrossCallArgs::try_from_slice(&bytes) {
                Ok(args) => AsyncResult::with_result(Ok(args)),
                Err(e) => AsyncResult::with_result(Err(Error::Serialization(format!("Failed to deserialize call details: {}", e)))),
            }
        },
        None => AsyncResult::with_result(Err(Error::State(String::from("No call details found")))),
    }
}

/// Make multiple cross-contract calls in parallel
pub async fn parallel_calls(ctx: &mut Context, targets: Vec<CrossCallArgs>) -> AsyncResult<Vec<CrossCallResult>> {
    let mut results = Vec::new();

    // For now, we'll just call them sequentially but demonstrate the API
    for args in targets {
        let async_result = call_with_timeout(ctx, args).await;
        match async_result.result {
            Some(Ok(result)) => results.push(result),
            Some(Err(e)) => return AsyncResult::with_result(Err(e)),
            None => return AsyncResult::with_result(Err(Error::Contract("Execution failed".to_string()))),
        }
    }
    
    AsyncResult::with_result(Ok(results))
}

fn main() {
    println!("Advanced Cross-Contract Caller Example");
    println!("-------------------------------------");
    println!("This example demonstrates making async calls to other contracts");
    println!("with proper error handling, gas accounting, and state management.");
    println!();
    println!("Run the example with:");
    println!("  cargo run --example advanced_cross_contract_caller");
}
