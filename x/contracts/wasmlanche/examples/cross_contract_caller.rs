use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    context::{Context, WasmlAddress},
    future::{AsyncResult, ContractCallResult, StateResult},
    public,
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

#[public]
pub async fn make_cross_contract_call(ctx: &mut Context, args: CallArgs) -> AsyncResult<CallResult> {
    // Store some state first
    let key = b"last_called";
    ctx.store_state(key, &args.target.to_bytes()).await?;
    
    // Make a cross-contract call
    let call_args = args.amount.to_le_bytes().to_vec();
    let result = ctx.call_contract(&args.target, "get_value", &call_args).await?;
    
    // Read the result
    let value = if !result.is_empty() {
        let bytes: [u8; 8] = result.try_into().map_err(|_| "Invalid result length")?;
        u64::from_le_bytes(bytes)
    } else {
        0
    };
    
    // Return the result
    AsyncResult::Ok(CallResult {
        success: true,
        value,
    })
}

#[public]
pub async fn get_last_called(ctx: &mut Context) -> StateResult<WasmlAddress> {
    let key = b"last_called";
    match ctx.get_state(key).await? {
        Some(bytes) => {
            let address = WasmlAddress::from_bytes(&bytes).map_err(|_| "Invalid address")?;
            StateResult::Ok(address)
        }
        None => StateResult::Err("No last called contract".into()),
    }
}
