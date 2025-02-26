// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::Context;

#[public]
pub fn execute(ctx: &mut Context) -> i64 {
    // Simulate some async work
    let mut counter = 0;
    for _ in 0..1000 {
        counter += 1;
    }
    counter
}
