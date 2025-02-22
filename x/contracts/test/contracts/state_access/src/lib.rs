// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::{Context, state::StateKey, state::StateAccess};
use wasmlanche::borsh::{self, BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
struct State {
    value: u64,
}

impl StateKey for State {
    fn get_key() -> Vec<u8> {
        b"state".to_vec()
    }
}

/// Initializes the contract with a name, symbol, and total supply.
#[public]
pub async fn store_state(context: &mut Context, value: u64) -> bool {
    let state = State { value };
    context.store_state(&state).await.is_ok()
}

#[public]
pub async fn get_state(context: &mut Context) -> Option<u64> {
    match context.get_state::<State>().await {
        Ok(Some(state)) => Some(state.value),
        _ => None,
    }
}

#[public]
pub async fn delete_state(context: &mut Context) -> bool {
    context.delete_state::<State>().await.is_ok()
}
