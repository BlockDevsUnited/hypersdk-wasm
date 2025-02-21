// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use wasmlanche::{prelude::public, Context};

#[public]
pub fn always_true(_: &mut Context) -> bool {
    true
}

#[public]
pub fn bench_add(context: &mut Context, a: u64, b: u64) -> u64 {
    a + b
}

#[public]
pub fn bench_sub(context: &mut Context, a: u64, b: u64) -> u64 {
    a - b
}

#[public]
pub fn bench_mul(context: &mut Context, a: u64, b: u64) -> u64 {
    a * b
}

#[public]
pub fn bench_div(context: &mut Context, a: u64, b: u64) -> u64 {
    a / b
}
