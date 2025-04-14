use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    context::Context,
    future::{AsyncResult, StateResult},
    error::Error,
    types::WasmlAddress,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct CallArgs {
    pub target: WasmlAddress,
    pub amount: u64,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct CallResult {
    pub success: bool,
    pub value: u64,
}

pub async fn make_cross_contract_call(ctx: &mut Context, args: CallArgs) -> AsyncResult<CallResult> {
    // Store some state first
    let key = b"last_called";
    // Use as_bytes() instead of to_bytes()
    let result = ctx.store_by_key(key, args.target.as_bytes().to_vec());
    match result {
        Err(e) => return AsyncResult::with_result(Err(e)),
        Ok(_) => {},
    }
    
    // Make a cross-contract call
    let call_args = args.amount.to_le_bytes().to_vec();
    
    // Use match instead of ? operator
    // Add None as the timeout parameter (no timeout)
    let result = match ctx.call_contract(&args.target, "get_value", &call_args, None).await {
        Ok(data) => data,
        Err(e) => return AsyncResult::with_result(Err(e)),
    };
    
    // Read the result
    let value = if !result.is_empty() {
        // Use match instead of ? operator
        match result.try_into() {
            Ok(bytes) => u64::from_le_bytes(bytes),
            Err(_) => return AsyncResult::with_result(Err(Error::Serialization(String::from("Invalid result format")))),
        }
    } else {
        0
    };
    
    // Return the result using the correct AsyncResult constructor
    AsyncResult::with_result(Ok(CallResult {
        success: true,
        value,
    }))
}

pub async fn get_last_called(ctx: &mut Context) -> StateResult<Option<WasmlAddress>> {
    let key = b"last_called";
    
    // We need to use get_by_key to access the raw bytes using a custom key
    match ctx.get_by_key(key) {
        Ok(Some(bytes)) => {
            // Use WasmlAddress::new or From trait instead of from_bytes
            if bytes.len() == 32 {
                let mut addr_bytes = [0u8; 32];
                addr_bytes.copy_from_slice(&bytes);
                // StateResult<Option<WasmlAddress>> means we need to return
                // a Result<Option<Option<WasmlAddress>>, Error>
                StateResult::with_result(Ok(Some(Some(WasmlAddress::new(addr_bytes)))))
            } else {
                StateResult::with_result(Err(Error::Serialization("Invalid address bytes".to_string())))
            }
        }
        Ok(None) => StateResult::with_result(Ok(Some(None))),
        Err(e) => StateResult::with_result(Err(e)),
    }
}

fn main() {
    println!("Cross-Contract Caller Example");
    println!("----------------------------");
    println!("This example demonstrates a contract that makes calls to other contracts.");
    println!("It stores the address of the last called contract and the result.");
    println!();
    println!("Run the example with:");
    println!("  cargo run --example cross_contract_caller");
}
