use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct TestState {
    value: u64,
}

#[no_mangle]
pub extern "C" fn test_function() -> u64 {
    42
}
