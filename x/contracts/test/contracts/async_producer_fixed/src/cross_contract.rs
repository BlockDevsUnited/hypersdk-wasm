// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use alloc::string::String;
use alloc::format;
use alloc::vec::Vec;
use sdk_macros::public;
use wasmlanche::Context;

#[public]
pub async fn call_consumer_async(ctx: &mut Context, consumer_id: Vec<u8>) -> i64 {
    // Call the consumer contract to read our produced value
    match ctx.call_contract_async(&consumer_id, "consume", &[]).await {
        Ok(result) => {
            match i64::try_from(result) {
                Ok(value) => value,
                Err(_) => -10, // Error converting result
            }
        },
        Err(_) => -11, // Error calling contract
    }
}

#[public]
pub fn start_call_consumer(ctx: &mut Context, consumer_id: Vec<u8>) -> String {
    // Start an async call to the consumer contract
    match ctx.call_contract_async(&consumer_id, "consume_async", &[]) {
        Ok(op_id) => op_id,
        Err(err) => format!("ERROR:{:?}", err),
    }
}
