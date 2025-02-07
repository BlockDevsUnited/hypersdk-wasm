// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::{public, state_schema, Context};

state_schema! {
    State => i64
}

/// Initializes the contract with a name, symbol, and total supply.
#[public]
pub async fn put(context: &mut Context, value: i64) {
    let state = State { value };
    context.store_state(&state.to_bytes(), &state.to_bytes())
        .await
        .expect("failed to store state");
}

#[public]
pub async fn get(context: &mut Context) -> Option<i64> {
    context.get_state(&State::key())
        .await
        .expect("failed to get state")
        .map(|state| state.value)
}

#[public]
pub async fn delete(context: &mut Context) -> Option<i64> {
    context.delete_state(&State::key())
        .await
        .expect("failed to delete state")
        .map(|state| state.value)
}
