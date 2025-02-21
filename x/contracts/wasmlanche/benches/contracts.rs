// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use borsh::{BorshSerialize, BorshDeserialize};
use wasmlanche_test::create_test_context;

#[derive(BorshSerialize, BorshDeserialize)]
struct State {
    value: u64,
}

fn bench_state_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut context = create_test_context();

    c.bench_function("store_state", |b| {
        b.iter(|| {
            rt.block_on(async {
                let state = State { value: black_box(42) };
                context.store_state(&state).await.unwrap();
            });
        });
    });

    c.bench_function("get_state", |b| {
        b.iter(|| {
            rt.block_on(async {
                let state: State = context.get_state().await.unwrap().unwrap();
                black_box(state);
            });
        });
    });
}

fn bench_balance_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut context = create_test_context();
    let addr = context.actor().clone();

    c.bench_function("transfer_balance", |b| {
        b.iter(|| {
            rt.block_on(async {
                let amount = black_box(100);
                let to = addr.clone();
                context.transfer(&addr, &to, amount).await.unwrap();
            });
        });
    });

    c.bench_function("get_balance", |b| {
        b.iter(|| {
            rt.block_on(async {
                let balance = context.get_balance(&addr).await.unwrap();
                black_box(balance);
            });
        });
    });
}

criterion_group!(benches, bench_state_operations, bench_balance_operations);
criterion_main!(benches);
