// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use wasmlanche::types::WasmlAddress;
use wasmlanche_test::Builder;

mod contracts;

fn call_contract(c: &mut Criterion) {
    let mut contract = contracts::Contract::new(Builder::new("test-crate"));
    let mut nft = contracts::Nft::new(Builder::new("test-crate"));
    let mut group = c.benchmark_group("call_contract");

    group.measurement_time(Duration::from_secs(10));
    
    // Basic function call benchmark
    group.bench_function("always_true", |b| b.iter(|| contract.always_true()));
    
    // State operation benchmarks
    let key = b"test_key".to_vec();
    let value = b"test_value".to_vec();
    group.bench_function("read_state", |b| {
        b.iter(|| contract.read_state(&key))
    });
    group.bench_function("write_state", |b| {
        b.iter(|| contract.write_state(&key, &value))
    });
    group.bench_function("delete_state", |b| {
        b.iter(|| contract.delete_state(&key))
    });

    // Balance operation benchmarks
    let address = WasmlAddress::new([1; 32]);
    group.bench_function("get_balance", |b| {
        b.iter(|| contract.get_balance(&address))
    });
    group.bench_function("set_balance", |b| {
        b.iter(|| contract.set_balance(&address, 1000))
    });

    // Event operation benchmarks
    let event = b"test_event".to_vec();
    group.bench_function("emit_event", |b| {
        b.iter(|| contract.emit_event(&event))
    });
    group.bench_function("get_events", |b| {
        b.iter(|| contract.get_events())
    });

    // Schema operation benchmarks
    let schema = b"test_schema".to_vec();
    group.bench_function("store_schema", |b| {
        b.iter(|| contract.store_schema(&schema))
    });
    group.bench_function("get_schema", |b| {
        b.iter(|| contract.get_schema())
    });

    // NFT operation benchmark
    group.bench_function("mint", |b| {
        b.iter(|| {
            let address = WasmlAddress::new([1; 32]);
            nft.mint(address, 1)
        })
    });

    // Parallel operation benchmarks
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("parallel_reads", size), size, |b, &size| {
            b.iter(|| {
                let mut contract = contracts::Contract::new(Builder::new("test-crate"));
                for i in 0..size {
                    let key = format!("key_{}", i).into_bytes();
                    contract.read_state(&key);
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("parallel_events", size), size, |b, &size| {
            b.iter(|| {
                let mut contract = contracts::Contract::new(Builder::new("test-crate"));
                for i in 0..size {
                    let event = format!("event_{}", i).into_bytes();
                    contract.emit_event(&event);
                }
            })
        });
    }

    group.finish();
}

criterion_group!(benches, call_contract);
criterion_main!(benches);
