use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    context::Context,
    future::AsyncResult,
    public,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ValueResponse {
    pub value: u64,
}

#[public]
pub async fn get_value(ctx: &mut Context, amount: u64) -> AsyncResult<u64> {
    // Store a record of this call
    let key = b"last_request_amount";
    ctx.store_state(key, &amount.to_le_bytes()).await?;
    
    // Return doubled amount
    AsyncResult::Ok(amount * 2)
}

#[public]
pub async fn get_last_request(ctx: &mut Context) -> AsyncResult<u64> {
    let key = b"last_request_amount";
    match ctx.get_state(key).await? {
        Some(bytes) => {
            let amount_bytes: [u8; 8] = bytes.try_into().map_err(|_| "Invalid amount bytes")?;
            AsyncResult::Ok(u64::from_le_bytes(amount_bytes))
        }
        None => AsyncResult::Ok(0),
    }
}
