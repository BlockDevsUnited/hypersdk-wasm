use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    context::Context,
    future::AsyncResult,
    error::Error,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ValueResponse {
    pub value: u64,
}

// Removed #[public] attribute
pub async fn get_value(ctx: &mut Context, amount: u64) -> AsyncResult<u64> {
    // Store a record of this call
    let key = b"last_request_amount";
    let bytes = amount.to_le_bytes().to_vec();
    match ctx.store_by_key(key, bytes) {
        Ok(_) => {},
        Err(e) => return AsyncResult::with_result(Err(e)),
    }
    
    // Return doubled amount
    AsyncResult::with_result(Ok(amount * 2))
}

// Removed #[public] attribute
pub async fn get_last_request(ctx: &mut Context) -> AsyncResult<u64> {
    let key = b"last_request_amount";
    match ctx.get_by_key(key) {
        Ok(Some(bytes)) => {
            match bytes.try_into() {
                Ok(amount_bytes) => AsyncResult::with_result(Ok(u64::from_le_bytes(amount_bytes))),
                Err(_) => AsyncResult::with_result(Err(Error::Serialization(String::from("Invalid amount bytes")))),
            }
        }
        Ok(None) => AsyncResult::with_result(Ok(0)),
        Err(e) => AsyncResult::with_result(Err(e)),
    }
}

fn main() {
    println!("Cross-Contract Callee Example");
    println!("----------------------------");
    println!("This example demonstrates a contract that receives calls from other contracts.");
    println!("It increments a counter and returns the value to the caller.");
    println!();
    println!("Run the example with:");
    println!("  cargo run --example cross_contract_callee");
}
