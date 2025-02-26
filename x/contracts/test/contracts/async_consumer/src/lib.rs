// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::Context;

const SHARED_KEY: &[u8] = b"shared_value";

#[public]
pub fn consume(ctx: &mut Context) -> i64 {
    // Read value from state
    if let Some(bytes) = ctx.get_state().unwrap().get(SHARED_KEY) {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&bytes);
        i64::from_le_bytes(buf)
    } else {
        0
    }
}
