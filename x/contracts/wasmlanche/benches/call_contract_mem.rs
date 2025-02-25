// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::hint::black_box;
use std::time::Duration;
use wasmlanche::types::WasmlAddress;
use wasmlanche_test::Builder;

mod contracts;

iai::main!(call_contract);

fn call_contract() {
    let builder = Builder::new("test-crate");
    let mut contract = contracts::Contract::new(builder);
    black_box(contract.always_true());
}
