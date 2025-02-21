// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::{Context, prelude::public, types::WasmlAddress};
use wasmlanche::borsh::{BorshSerialize, BorshDeserialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ComplexReturn {
    pub account: WasmlAddress,
    pub max_units: u64,
}

#[public]
pub async fn get_value(ctx: &mut Context) -> ComplexReturn {
    let account = ctx.actor().clone();
    ComplexReturn {
        account,
        max_units: 1000,
    }
}
