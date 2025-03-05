//! Advanced Cross-Contract Caller Example
//! 
//! This contract demonstrates making async calls to other contracts
//! with proper timeout handling, error management, and gas accounting.

use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    context::{Context, WasmlAddress},
    error::Error,
    future::AsyncResult,
    public,
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
#[public]
pub async fn call_with_timeout(ctx: &mut Context, args: CrossCallArgs) -> AsyncResult<CrossCallResult> {
    // Record initial gas to calculate usage later
    let initial_gas = ctx.remaining_gas();
    
    // Store call details in state for later retrieval
    let key = b"last_call_details";
    let encoded = borsh::to_vec(&args).map_err(|e| Error::Serialization(format!("Failed to serialize args: {}", e)))?;
    ctx.store_state(key, &encoded).await?;
    
    // If simulating error, fail immediately
    if args.simulate_error {
        return AsyncResult::with_result(Err(Error::Contract(String::from("Simulated error in cross-contract call"))));
    }
    
    // Make the cross-contract call with timeout
    let call_args = args.amount.to_le_bytes().to_vec();
    let start_time = ctx.current_timestamp().await?;
    
    let result = match ctx.call_contract(&args.target, "get_value", &call_args, args.timeout_ms).await {
        Ok(data) => {
            // Parse the result
            let value = if !data.is_empty() {
                let bytes: [u8; 8] = data.try_into().map_err(|_| Error::Serialization("Invalid result length".to_string()))?;
                u64::from_le_bytes(bytes)
            } else {
                0
            };
            
            // Calculate execution time
            let end_time = ctx.current_timestamp().await?;
            let execution_time = end_time.saturating_sub(start_time);
            
            // Calculate gas used
            let final_gas = ctx.remaining_gas();
            let gas_used = initial_gas.saturating_sub(final_gas);
            
            // Prepare the successful result
            CrossCallResult {
                success: true,
                value,
                gas_used,
                execution_time_ms: execution_time,
            }
        },
        Err(e) => {
            // Handle timeout or other errors
            CrossCallResult {
                success: false,
                value: 0,
                gas_used: initial_gas.saturating_sub(ctx.remaining_gas()),
                execution_time_ms: 0,
            }
        }
    };
    
    AsyncResult::with_result(Ok(result))
}

/// Get the details of the last call made
#[public]
pub async fn get_last_call_details(ctx: &mut Context) -> AsyncResult<CrossCallArgs> {
    let key = b"last_call_details";
    match ctx.get_state(key).await? {
        Some(bytes) => {
            let args = borsh::from_slice(&bytes)
                .map_err(|e| Error::Serialization(format!("Failed to deserialize call details: {}", e)))?;
            AsyncResult::with_result(Ok(args))
        },
        None => AsyncResult::with_result(Err(Error::State("No call details found".to_string()))),
    }
}

/// Make multiple cross-contract calls in parallel
#[public]
pub async fn parallel_calls(ctx: &mut Context, targets: Vec<CrossCallArgs>) -> AsyncResult<Vec<CrossCallResult>> {
    let mut results = Vec::new();

    // For now, we'll just call them sequentially but demonstrate the API
    for args in targets {
        let result = call_with_timeout(ctx, args).await?;
        results.push(result);
    }
    
    AsyncResult::with_result(Ok(results))
}
