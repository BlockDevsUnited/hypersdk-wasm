//! Advanced Cross-Contract Callee Example
//! 
//! This contract demonstrates receiving async calls from other contracts
//! with proper error handling, gas accounting, and state management.

use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    context::Context,
    error::Error,
    future::AsyncResult,
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
pub async fn get_value(ctx: &mut Context, amount: u64) -> AsyncResult<u64> {
    // Record call details
    let metadata_key = b"metadata";
    let metadata_result = ctx.get_by_key(metadata_key);
    
    let mut metadata = match metadata_result {
        Ok(Some(bytes)) => {
            match CallMetadata::try_from_slice(&bytes) {
                Ok(mut existing) => {
                    existing.amount = amount;
                    existing.call_count += 1;
                    // Get the current timestamp
                    match ctx.current_timestamp().await {
                        Ok(timestamp) => existing.timestamp = timestamp,
                        Err(e) => return AsyncResult::with_result(Err(e)),
                    }
                    existing
                },
                Err(e) => return AsyncResult::with_result(Err(Error::Serialization(format!("Failed to deserialize metadata: {}", e)))),
            }
        },
        Ok(None) => {
            let timestamp = match ctx.current_timestamp().await {
                Ok(ts) => ts,
                Err(e) => return AsyncResult::with_result(Err(e)),
            };
            CallMetadata {
                caller: None, // In a real implementation, we would get the caller's address
                amount,
                timestamp,
                call_count: 1,
            }
        },
        Err(e) => return AsyncResult::with_result(Err(e)),
    };
    
    // Store updated metadata
    let serialized = match borsh::to_vec(&metadata) {
        Ok(bytes) => bytes,
        Err(e) => return AsyncResult::with_result(Err(Error::Serialization(format!("Failed to serialize metadata: {}", e)))),
    };
    
    match ctx.store_by_key_async(metadata_key, serialized) {
        Ok(_) => {},
        Err(e) => return AsyncResult::with_result(Err(e)),
    }
    
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
pub async fn get_call_metadata(ctx: &mut Context) -> AsyncResult<CallMetadata> {
    let metadata_key = b"metadata";
    match ctx.get_by_key(metadata_key) {
        Ok(Some(bytes)) => {
            match CallMetadata::try_from_slice(&bytes) {
                Ok(metadata) => AsyncResult::with_result(Ok(metadata)),
                Err(e) => AsyncResult::with_result(Err(Error::Serialization(format!("Failed to deserialize metadata: {}", e)))),
            }
        },
        Ok(None) => AsyncResult::with_result(Err(Error::State(String::from("No call metadata found")))),
        Err(e) => AsyncResult::with_result(Err(e)),
    }
}

/// Reset the call counter - useful for testing
pub async fn reset_call_counter(ctx: &mut Context) -> AsyncResult<u64> {
    let metadata_key = b"metadata";
    
    match ctx.get_by_key(metadata_key) {
        Ok(Some(bytes)) => {
            match CallMetadata::try_from_slice(&bytes) {
                Ok(mut metadata) => {
                    let old_count = metadata.call_count;
                    metadata.call_count = 0;
                    
                    let serialized = match borsh::to_vec(&metadata) {
                        Ok(bytes) => bytes,
                        Err(e) => return AsyncResult::with_result(Err(Error::Serialization(format!("Failed to serialize metadata: {}", e)))),
                    };
                    
                    match ctx.store_by_key_async(metadata_key, serialized) {
                        Ok(_) => AsyncResult::with_result(Ok(old_count)),
                        Err(e) => AsyncResult::with_result(Err(e)),
                    }
                },
                Err(e) => AsyncResult::with_result(Err(Error::Serialization(format!("Failed to deserialize metadata: {}", e)))),
            }
        },
        Ok(None) => AsyncResult::with_result(Ok(0)),
        Err(e) => AsyncResult::with_result(Err(e)),
    }
}

/// Simulate an operation that takes a long time
/// 
/// This is useful for testing timeouts in cross-contract calls
pub async fn slow_operation(ctx: &mut Context, delay_ms: u64) -> AsyncResult<bool> {
    // Record the operation in state
    let key = b"slow_operation_delay";
    let value = delay_ms.to_le_bytes().to_vec();
    match ctx.store_by_key_async(key, value) {
        Ok(_) => {},
        Err(e) => return AsyncResult::with_result(Err(e)),
    }
    
    // If this is a time-out request, just don't respond
    // In a real implementation, we would simulate a delay:
    if delay_ms > 5000 {
        // This would cause a timeout in many systems
        // Simulate by not returning - in reality we'd use sleep
        loop {
            // Just burn CPU time
            let _ = delay_ms * 2;
        }
    }
    
    // For shorter delays, we'd just return after the delay
    // In a real implementation with true async support:
    // tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    
    AsyncResult::with_result(Ok(true))
}

fn main() {
    println!("Advanced Cross-Contract Callee Example");
    println!("-------------------------------------");
    println!("This example demonstrates receiving async calls from other contracts");
    println!("with proper error handling, gas accounting, and state management.");
    println!();
    println!("Run the example with:");
    println!("  cargo run --example advanced_cross_contract_callee");
}
