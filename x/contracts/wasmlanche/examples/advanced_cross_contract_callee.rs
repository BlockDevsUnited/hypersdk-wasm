//! Advanced Cross-Contract Callee Example
//! 
//! This contract demonstrates receiving async calls from other contracts
//! with proper error handling, gas accounting, and state management.

use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    context::Context,
    error::Error,
    future::AsyncResult,
    public,
    state::StateKey,
};

/// A more complex data structure to demonstrate serialization
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct CallMetadata {
    /// The caller's contract address (if available)
    pub caller: Option<[u8; 32]>,
    
    /// The amount passed in the call
    pub amount: u64,
    
    /// Timestamp of when the call was received
    pub timestamp: u64,
    
    /// Total calls received
    pub call_count: u64,
}

impl StateKey for CallMetadata {
    fn key(&self) -> Vec<u8> {
        b"call_metadata".to_vec()
    }
    
    fn key_static() -> Vec<u8> {
        b"call_metadata".to_vec()
    }
}

impl Default for CallMetadata {
    fn default() -> Self {
        Self {
            caller: None,
            amount: 0,
            timestamp: 0,
            call_count: 0,
        }
    }
}

/// Process a value and return a result
/// 
/// This function demonstrates proper error handling, state management,
/// and gas accounting when receiving cross-contract calls.
#[public]
pub async fn get_value(ctx: &mut Context, amount: u64) -> AsyncResult<u64> {
    // Record call details
    let mut metadata = match ctx.get_state::<CallMetadata>().await? {
        Some(mut existing) => {
            existing.amount = amount;
            existing.call_count += 1;
            // Get the current timestamp
            existing.timestamp = ctx.current_timestamp().await?;
            existing
        },
        None => CallMetadata {
            caller: None, // In a real implementation, we would get the caller's address
            amount,
            timestamp: ctx.current_timestamp().await?,
            call_count: 1,
        },
    };
    
    // Store updated metadata
    ctx.store_state(&metadata).await?;
    
    // Simulate work - in a real contract this might be a complex calculation
    // that consumes gas and might need to be bounded
    
    // Delay processing based on amount - higher amounts take longer
    // This is a simple simulation of work that might take time
    let processing_time = amount.min(1000);
    
    // In a real delayed implementation, we might do:
    // tokio::time::sleep(Duration::from_millis(processing_time)).await;
    
    // Check if we should fail the request (for testing error handling)
    if amount == 0xDEAD {
        return AsyncResult::with_result(Err(Error::Contract(String::from("Requested error simulation"))));
    }
    
    // Return doubled amount
    AsyncResult::with_result(Ok(amount * 2))
}

/// Get stored call metadata
#[public]
pub async fn get_call_metadata(ctx: &mut Context) -> AsyncResult<CallMetadata> {
    match ctx.get_state::<CallMetadata>().await? {
        Some(metadata) => AsyncResult::with_result(Ok(metadata)),
        None => AsyncResult::with_result(Err(Error::State(String::from("No call metadata found")))),
    }
}

/// Reset the call counter - useful for testing
#[public]
pub async fn reset_call_counter(ctx: &mut Context) -> AsyncResult<u64> {
    match ctx.get_state::<CallMetadata>().await? {
        Some(mut metadata) => {
            let old_count = metadata.call_count;
            metadata.call_count = 0;
            ctx.store_state(&metadata).await?;
            AsyncResult::with_result(Ok(old_count))
        },
        None => AsyncResult::with_result(Ok(0)),
    }
}

/// Simulate a slow or hanging operation
/// 
/// This is used to test timeout handling in the caller contract
#[public]
pub async fn slow_operation(ctx: &mut Context, delay_ms: u64) -> AsyncResult<u64> {
    // Record the operation in state
    let key = b"slow_operation_delay";
    let value = delay_ms.to_le_bytes().to_vec();
    ctx.store_state(key, &value).await?;
    
    // If this is a time-out request, just don't respond
    // In a real implementation, we would simulate a delay:
    // tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    
    // For testing purposes:
    // If delay is extremely large, we simulate a hanging operation
    if delay_ms > 10000 {
        // In a real implementation, this would never actually return
        AsyncResult::with_result(Ok(42))
    } else {
        // Otherwise we complete normally after the simulated delay
        AsyncResult::with_result(Ok(delay_ms))
    }
}
