// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::{
    Context,
    state::{StateKey, StateAccess},
};
use borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct State {
    value: u64,
}

impl StateKey for State {
    fn get_key() -> Vec<u8> {
        b"state".to_vec()
    }
}

#[no_mangle]
pub async fn store(context: &mut Context, value: u64) {
    let state = State { value };
    context.store_state(&state).await.unwrap();
}

#[no_mangle]
pub async fn remove(context: &mut Context) {
    context.delete_state::<State>().await.unwrap();
}

#[no_mangle]
pub async fn load(context: &mut Context) -> Option<u64> {
    match context.get_state::<State>().await {
        Ok(Some(state)) => Some(state.value),
        _ => None
    }
}

#[no_mangle]
pub async fn load_or_default(context: &mut Context) -> u64 {
    match context.get_state::<State>().await {
        Ok(Some(state)) => {
            context.delete_state::<State>().await.unwrap();
            state.value
        }
        _ => 0
    }
}
