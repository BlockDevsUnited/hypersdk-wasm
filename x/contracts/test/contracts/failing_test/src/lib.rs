// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::Context;

#[public]
pub fn fail(_ctx: &mut Context) -> i64 {
    panic!("Simulated contract failure");
}
