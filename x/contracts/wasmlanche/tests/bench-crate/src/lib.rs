// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use sdk_macros::public;
use wasmlanche::Context;

#[public]
pub fn always_true(_: &mut Context) -> bool {
    true
}
