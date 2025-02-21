// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wasmlanche::types::Address;
use wasmlanche_test::create_test_context;

fn bench_call_contract(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut context = create_test_context();
    let target = Address::from([0u8; 33]);

    c.bench_function("call_contract", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = context
                    .call_contract(&target.as_bytes(), "test", &[], black_box(1000))
                    .await
                    .unwrap();
                black_box(result);
            });
        });
    });
}

criterion_group!(benches, bench_call_contract);
criterion_main!(benches);
