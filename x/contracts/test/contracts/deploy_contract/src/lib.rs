// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::{Context, types::{ContractId, WasmlAddress}};

#[public]
pub async fn deploy(ctx: &mut Context, contract_id: ContractId) -> WasmlAddress {
    ctx.deploy(contract_id).await.unwrap()
}
