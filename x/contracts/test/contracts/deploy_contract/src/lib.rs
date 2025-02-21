// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::{Context, prelude::public, types::WasmlAddress};

#[public]
pub async fn deploy_contract(ctx: &mut Context, contract_id: [u8; 32]) -> WasmlAddress {
    let target = contract_id.to_vec();
    let args = vec![];
    ctx.call_contract(&target, "deploy", &args, 1000000).await.unwrap();
    ctx.actor().clone()
}
