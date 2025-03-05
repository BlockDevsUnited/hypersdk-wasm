use wasmlanche::{
    borsh::{BorshDeserialize, BorshSerialize},
    context::{Context, WasmlAddress},
    future::{AsyncResult, ContractCallResult, StateResult, UnitResult},
    public,
    state::StateKey,
    error::Error,
};

// Example of using AsyncResult for general async operations
#[public]
pub async fn process_value(ctx: &mut Context, value: u64) -> Result<u64, Error> {
    // Simulate some async processing
    let doubled = value * 2;
    
    // Store the processed value for later retrieval
    let key = b"processed_value";
    ctx.store_state(key, &doubled.to_le_bytes()).map_err(|e| Error::State(e.to_string()))?;
    
    // Return the processed value
    Ok(doubled)
}

// Example of using StateResult for operations focused on state
#[public]
pub async fn get_processed_value(ctx: &mut Context) -> Result<u64, Error> {
    let key = b"processed_value";
    match ctx.get_state::<[u8; 8]>(key).map_err(|e| Error::State(e.to_string()))? {
        Some(bytes) => {
            let value_bytes = bytes;
            Ok(u64::from_le_bytes(value_bytes))
        }
        None => Err(Error::State("No processed value found".into())),
    }
}

// Example of using ContractCallResult for cross-contract calls
#[public]
pub async fn call_other_contract(ctx: &mut Context, target: WasmlAddress, value: u64) -> Result<u64, Error> {
    // Prepare the call arguments
    let args = value.to_le_bytes().to_vec();
    
    // Make the cross-contract call
    // In a real implementation, this would be a real contract call
    // let result = ctx.call_contract(target, "multiply", &args).await?;
    
    // For this example, we'll just return a mock value
    let result_value = value * 2;
    
    // Return the result
    Ok(result_value)
}

// Example of using UnitResult for operations that don't return a value
#[public]
pub async fn log_event(ctx: &mut Context, message: String) -> Result<(), Error> {
    // Log an event with the message
    // ctx.emit_event("log", message.as_bytes()).await?;
    
    // Return success
    Ok(())
}

// This struct demonstrates complex serialization across async operations
#[derive(BorshSerialize, BorshDeserialize)]
pub struct ComplexData {
    pub name: String,
    pub value: u64,
    pub is_valid: bool,
}

impl StateKey for ComplexData {
    fn get_key() -> &'static [u8] {
        b"complex_data"
    }
}

impl Default for ComplexData {
    fn default() -> Self {
        Self {
            name: String::new(),
            value: 0,
            is_valid: false,
        }
    }
}

#[public]
pub async fn store_complex_data(ctx: &mut Context, data: ComplexData) -> Result<(), Error> {
    // Serialize and store the complex data
    ctx.store_state(&data).map_err(|e| Error::State(e.to_string()))?;
    
    // Since emit_event is not available, let's use a workaround
    // In a real implementation, you would emit an event
    // ctx.emit_event("data_stored", format!("Stored data for: {}", data.name).as_bytes()).await?;
    
    Ok(())
}

#[public]
pub async fn get_complex_data(ctx: &mut Context) -> Result<ComplexData, Error> {
    // Retrieve the complex data
    match ctx.get_state::<ComplexData>() {
        Ok(Some(data)) => Ok(data),
        Ok(None) => Err(Error::State("No complex data found".into())),
        Err(e) => Err(Error::State(e.to_string())),
    }
}
