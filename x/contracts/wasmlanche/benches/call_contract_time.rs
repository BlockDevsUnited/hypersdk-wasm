// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;
use wasmlanche::types::WasmlAddress;
use wasmlanche_test::Builder;

mod contracts;

fn call_contract(c: &mut Criterion) {
    let builder = Builder::new("test-crate");
    let mut contract = contracts::Contract::new(builder);
    let mut group = c.benchmark_group("call_contract");

    group.measurement_time(Duration::from_secs(10));
    group.bench_function("always_true", |b| b.iter(|| contract.always_true()));
    group.finish();
}

criterion_group!(benches, call_contract);
criterion_main!(benches);
